//! Sequential implementation of a single-use runtime with reference-counted activators.

use crossbeam::deque;
use std::thread;
use std::marker::PhantomData;
use std::sync::{Arc,Mutex}; // ,Condvar retiré
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use api::prelude::*;

use parallel::port::RcPort;

/* 
Implémentation d'un compteur atomique 
inspiré de : https://docs.rs/atomic-counter/1.0.1/atomic_counter/trait.AtomicCounter.html
*/

pub trait CompteurAtomic: Send + Sync {
    type PrimitiveType;
    fn inc(&self) -> Self::PrimitiveType;
    fn add(&self, amount: Self::PrimitiveType) -> Self::PrimitiveType;
    fn get(&self) -> Self::PrimitiveType;
}

pub struct Compteur(AtomicUsize);

impl Compteur {
    pub fn new(initial_count: usize) -> Compteur {
        Compteur(AtomicUsize::new(initial_count))
    }
}

impl CompteurAtomic for Compteur {
    type PrimitiveType = usize;
    fn inc(&self) -> usize {
        self.add(1)
    }
    fn add(&self, amount: usize) -> usize {
        self.0.fetch_add(amount, SeqCst)
    }
    fn get(&self) -> usize {
        self.0.load(SeqCst)
    }
}


/// The inner structure for a single-use activator, containing the pending count and the node
/// handle.
struct RcActivatorInner<'r> {
    /// The pending count.
    pending: AtomicUsize, // seqcst

    /// The underlying node to schedule.  Note that we store a Box of a trait object here, instead
    /// of using a type parameter and embedding the node in the structure.  This is because of a
    /// Rust limitation which prevents us from calling a method with `self` as argument on a trait
    /// object -- the same reason why we use `NodeBox`.  Unfortunately, that trick only works for
    /// `Box`, which the Rust compiler has special knowledge of -- so instead we use an extra level
    /// of indirection and put a box here.
    handle: Box<RuntimeNode<'r>>,
}

impl<'r> RcActivatorInner<'r> {
    fn new<N: NodeBox<RuntimeLoc<'r>> + Send + Sync + 'r>(node: N) -> Self { //+sync ?
        RcActivatorInner {
            pending: AtomicUsize::new(0),
            handle: Box::new(node),
        }
    }
}

/// A reference-counted, single-use activator.
///
/// The activator contains a handle to a node, as well as a counter for the number of activations
/// remaining before the node should be scheduled.
///
/// When the node is finalized, the counter is set to the number of activators created.  It is
/// decremented by one on each activation, and the node is scheduled when the counter reaches zero.
/// Since activating consumes an activator, we ensure that the pending count only ever reaches zero
/// if all activators have been called.
pub struct RcActivator<'r> {
    inner: Arc<RcActivatorInner<'r>>,
}

impl<'r> ActivatorOnce<RuntimeLoc<'r>> for RcActivator<'r> {
    fn activate_once(self, scheduler: &mut RuntimeLoc<'r>) {
        if self.inner.pending.fetch_sub(1,SeqCst) == 1 {
            scheduler.schedule(Arc::try_unwrap(self.inner).ok().unwrap().handle)
        }
    }
}

impl<'r> ActivatorOnce<Toexec<'r>> for RcActivator<'r> {
    fn activate_once(self, scheduler: &mut Toexec<'r>) {
        if self.inner.pending.fetch_sub(1,SeqCst) == 1 {
            scheduler.ready.push(Arc::try_unwrap(self.inner).ok().unwrap().handle)
        }
    }
}

/// A builder for single-use nodes.  Allow creation of activators and arms them when finalized.
///
/// Note that once the builder is created, no modifications to the node are permitted (the builder
/// does not implement the `NodeBorrowMut` trait).  This is due to the fact that we need to store a
/// (boxed) trait object inside the activator in order to be able to call it later using
/// `execute_box`; see the documentation on `RcActivatorInner`.
pub struct RcBuilder<'r, N> {
    inner: Arc<RcActivatorInner<'r>>,
    _marker: PhantomData<*const N>,
    num_activators: usize,
}

impl<'r, N: NodeBox<RuntimeLoc<'r>> + Send + Sync + 'r> RcBuilder<'r, N> {  //MMM
    fn new(node: N) -> Self {
        RcBuilder {
            inner: Arc::new(RcActivatorInner::new(node)),
            _marker: PhantomData,
            num_activators: 0,
        }
    }
}

impl<'r, N: NodeBox<RuntimeLoc<'r>> + Send + 'r> NodeBuilder<Toexec<'r>> // + Sync ?
    for RcBuilder<'r, N>
{
    type Node = N;
    fn add_activator(&mut self) -> RcActivator<'r> {
        self.num_activators += 1;

        RcActivator {
            inner: self.inner.clone(),
        }
    }
    fn finalize(&mut self, _runtime: &mut Toexec<'r>) { // MODIFIÉ
        self.inner.pending.store(self.num_activators,SeqCst);
    }
}

impl<'r, N: NodeBox<RuntimeLoc<'r>> + Send + 'r> NodeBuilder<RuntimeLoc<'r>> // + Sync ?
    for RcBuilder<'r, N>
{
    type Node = N;
    fn add_activator(&mut self) -> RcActivator<'r> {
        self.num_activators += 1;

        RcActivator {
            inner: self.inner.clone(),
        }
    }
    fn finalize(&mut self, _runtime: &mut RuntimeLoc<'r>) { // MODIFIÉ
        self.inner.pending.store(self.num_activators,SeqCst);
    }
}

// The type of nodes manipulated by the sequential single-use runtime.

type RuntimeNode<'r> = dyn NodeBox<RuntimeLoc<'r>> + Send + Sync + 'r;

pub struct Toexec<'r> {
    pub ready: Vec<Box<RuntimeNode<'r>>>,
}

pub struct RuntimeLoc<'r> {
    ready: deque::Worker<Box<RuntimeNode<'r>>>,
    stealers: Vec<deque::Stealer<Box<RuntimeNode<'r>>>>,
    // condvar: Arc<Condvar> // la méthode essayée avec des signaux ne fonctionne pas
}

impl<'r> Toexec<'r> {
    pub fn new() -> Self {
        Toexec { ready: Vec::new() }
    }

    pub fn execute(&mut self, k: usize) {    	
        // création de la variable de condition
	    //let syncr = &(Mutex::new( () ),Arc::new(Condvar::new())); // la méthode essayée avec des signaux ne fonctionne pas
        //let n = Compteur::new(0);

        // création des fifos
        let mut fifos = Vec::new();
	    let mut stealers = Vec::new();

        for _ in 0..k {
	        let fs = deque::fifo();
            fifos.push(fs.0);
	        stealers.push(fs.1);
        }

        // création des threads et runtimes associées
        crossbeam::scope(|scope| {
            for i in 0..(k) {
                let j = i.clone();

		        //let (ref _lock, ref cvar) = *syncr.clone();
                let ready_j = fifos.pop().unwrap();
                
                if i == 0 {
                    for w in self.ready.drain(..) {
                        ready_j.push(w)
                    }
                }
                
                let mut stealers_j = Vec::new();
                
                // l'ordre des stealers n'est pas "naturelle" pour que tout le monde ne vole pas au premier
                for w in (j + 1)..k {
                    stealers_j.push(stealers[w].clone());
                }

                for w in 0..j {
                    stealers_j.push(stealers[w].clone());
                }
		
                //let nref = &n;
                scope.spawn(move || {

                    let mut runtime_loc = RuntimeLoc {
                        ready: ready_j,
                        stealers: stealers_j,
			            //condvar: cvar.clone(),
                    };

                    //let n = Arc::clone(nref);
                    //println!("{}",nref.get());
                    
                    loop {
                        match runtime_loc.ready.pop() {
                            Some(t) => t.execute_box(&mut runtime_loc),
                            None => {
                                let mut i = 0;
                                let mut tour = Arc::new(Compteur::new(0));
                                loop {
                                    match runtime_loc.stealers[i].steal() {
                                        Some(t) => {
					                        t.execute_box(&mut runtime_loc);
					                        break
					                    },
                                        None => (),
                                    }
                                    i = (i + 1) % (k-1);

                                    if i == 0{
                                        if tour.get()==10{
                                            return;
                                        }
                                        else{
                                            tour.inc();
                                            thread::yield_now();
                                        }
                                    }
/*                                      // on attend qu'un schedule soit appelé

				                        let mut go = lock.lock().unwrap();
                                        println!("%");
                                        
                                        //n.inc();
                                        //kbis.inc();

                                        //if n.get() == kbis {
                                        //    return;
                                        //}

                                        let _ = cvar.wait(go).unwrap();
                                        
                                        //if q == 1 { // IMPORTANT <- Comment lire le contenu du CVAR ?
                                            //return;
                                        //}
                                        println!("p");
                                        //kbis.inc();
				                    }
*/                              
                                }
                            }
                        }
                    }
                });
            }
        });
    }
}

impl<'r> Scheduler for RuntimeLoc<'r> { 
    type Handle = Box<RuntimeNode<'r>>;

    fn schedule(&mut self, handle: Self::Handle) {
        self.ready.push(handle);
	    //self.condvar.notify_all()
    }
}

impl<'r> GraphSpec for Toexec<'r> {
    type Activator = RcActivator<'r>;
}


impl<'r, N: NodeBox<RuntimeLoc<'r>> + Send + Sync  + 'r> NodeSpec<N> for Toexec<'r> {
    type Builder = RcBuilder<'r, N>;

    fn node(&self, node: N) -> Self::Builder {
        RcBuilder::new(node)
    }
}

impl<'r, T: Default + 'r> PortSpec<T> for Toexec<'r> {
    type Port = RcPort<Mutex<T>>;

    fn port(&self, init: T) -> Self::Port {
        RcPort::new(Mutex::new(init))
    }
}

impl<'r> GraphSpec for RuntimeLoc<'r> {
    type Activator = RcActivator<'r>;
}


impl<'r, N: NodeBox<RuntimeLoc<'r>> + Send + Sync  + 'r> NodeSpec<N> for RuntimeLoc<'r> {
    type Builder = RcBuilder<'r, N>;

    fn node(&self, node: N) -> Self::Builder {
        RcBuilder::new(node)
    }
}

impl<'r, T: Default + 'r> PortSpec<T> for RuntimeLoc<'r> {
    type Port = RcPort<Mutex<T>>;

    fn port(&self, init: T) -> Self::Port {
        RcPort::new(Mutex::new(init))
    }
}