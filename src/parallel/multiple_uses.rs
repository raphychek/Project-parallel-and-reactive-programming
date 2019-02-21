//! A runtime which allows executing nodes multiple times using reference-counted activators.
//!
//! WARNING: This runtime is implemented using dynamic reference counting.  This makes it somewhat
//! dubious for its intended purpose of being able to reuse nodes through dependency cycles, as it
//! prevents the memory from ever being de-allocated.  This can be fixed relatively easily by
//! replacing the `Rc` implementations with a custom version using references instead; however,
//! this requires to statically allocate the inner structures as well and would cause a fair amount
//! of boring bookkeeping.  Since this is out of scope for the project, we'll accept the memory
//! leaks; but if you are interested, you can try to make this implementation leak-free.  You can
//! use an [arena](https://docs.rs/typed-arena/1.4.1/typed_arena/struct.Arena.html) to store the
//! data in a buffer, and may also be interested in using the `RefPort` from the `common` module
//! instead of `RcPort`.

use api::prelude::*;
use common::prelude::*;

use crossbeam::deque;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};
use std::thread;

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



/// The inner structure for the iterator.  This include a handle to the node, as well as a pending
/// count with interior mutability.  Contrary to the `single_use` implementation, we also use
/// interior mutability for the handle because we need to be able to access the handle while there
/// are still other references to the inner structure (hence the reusable nature).
#[derive(Debug)]
struct RcActivatorInner<H: ?Sized> {
    /// The pending count.  If 0, there is currently a builder or a handle pointing to the node.
    pending: AtomicUsize,
    /// The initial pending count to reset to.  This includes the handle.
    initial: AtomicUsize,
    /// The underlying node to schedule.
    handle: Mutex<H>,
}

impl<H> RcActivatorInner<H> {
    fn new(node: H) -> Self {
        RcActivatorInner {
            pending: AtomicUsize::new(0),
            initial: AtomicUsize::new(1),
            handle: Mutex::new(node),
        }
    }
}

impl<H: ?Sized> RcActivatorInner<H> {
    /// Rearm the activation structure with a new pending count. This should only be called when
    /// the activator was depleted.
    fn rearm(&self) {
        let initial = self.initial.load(SeqCst);
        assert!(self.pending.swap(initial, SeqCst) == 0);
    }

    /// Decrement the pending count and return the new pending count.
    fn decrement_pending(&self) -> usize {
        let old_pending = self.pending.fetch_sub(1, SeqCst);
        assert!(old_pending > 0);
        old_pending - 1
    }
}

/// A reference-counted, reusable activator.
///
/// The activator contains a handle to a node, a counter for the number of activations
/// remaining before the node should be scheduled, and the total number of activators.
///
/// When the node is finalized, the counter is set to the total number of activators.  It is
/// decremented by one on each activation, and the node is scheduled when the counter reaches zero.
#[derive(Debug)]
pub struct RcActivator<H: ?Sized> {
    inner: Arc<RcActivatorInner<H>>,
}

/// A default activator which schedules a panicking node.  This can be used as a placeholder
/// activator when the target node is not yet known.  Note that trying to activate this will
/// already trigger a panic in `decrement_pending` since it never gets armed.
impl<'r> Default for RcActivator<RuntimeNode<'r>> {
    fn default() -> Self {
        RcActivator {
            inner: Arc::new(RcActivatorInner::new(UninitializedNode)),
        }
    }
}

impl<'r> ActivatorOnce<RuntimeLoc<'r>> for RcActivator<RuntimeNode<'r>> {
    fn activate_once(self, scheduler: &mut RuntimeLoc<'r>) {
        if self.inner.decrement_pending() == 0 {
            scheduler.schedule(RcHandle { inner: self.inner })
        }
    }
}

impl<'r> ActivatorOnce<Toexec<'r>> for RcActivator<RuntimeNode<'r>> {
    fn activate_once(self, scheduler: &mut Toexec<'r>) {
        if self.inner.decrement_pending() == 0 {
            scheduler.schedule(RcHandle { inner: self.inner })
        }
    }
}

impl<'r> ActivatorMut<RuntimeLoc<'r>> for RcActivator<RuntimeNode<'r>> {
    fn activate_mut(&mut self, scheduler: &mut RuntimeLoc<'r>) {
        Activator::activate(self, scheduler)
    }
}

impl<'r> ActivatorMut<Toexec<'r>> for RcActivator<RuntimeNode<'r>> {
    fn activate_mut(&mut self, scheduler: &mut Toexec<'r>) {
        Activator::activate(self, scheduler)
    }
}

impl<'r> Activator<RuntimeLoc<'r>> for RcActivator<RuntimeNode<'r>> {
    fn activate(&self, scheduler: &mut RuntimeLoc<'r>) {
        if self.inner.decrement_pending() == 0 {
            scheduler.schedule(RcHandle {
                inner: self.inner.clone(),
            })
        }
    }
}

impl<'r> Activator<Toexec<'r>> for RcActivator<RuntimeNode<'r>> {
    fn activate(&self, scheduler: &mut Toexec<'r>) {
        if self.inner.decrement_pending() == 0 {
            scheduler.schedule(RcHandle {
                inner: self.inner.clone(),
            })
        }
    }
}

/// A node handle.  This is the structured used to actually schedule nodes.  A single handle to a
/// given node should ever exist, and it can only exist when the node's pending count is 0.
#[derive(Debug)]
pub struct RcHandle<H: ?Sized> {
    inner: Arc<RcActivatorInner<H>>,
}

impl<S, H: NodeMut<S> + ?Sized> NodeOnce<S> for RcHandle<H>
where
    RcActivator<H>: ActivatorOnce<S>,
{
    /// Execute the guard.  This consumes the guard and re-arm the activators, which allows the
    /// node to be executed again later.
    fn execute_once(self, scheduler: &mut S) {
        self.inner.rearm();
        self.inner.handle.lock().unwrap().execute_mut(scheduler);
        RcActivator { inner: self.inner }.activate_once(scheduler);
    }
}

/// A builder for reusable nodes.  Allow creation of activators and arms them when finalized.
#[derive(Debug)]
pub struct RcBuilder<N> {
    inner: Arc<RcActivatorInner<N>>,
    _marker: PhantomData<*const N>,
    num_activators: usize,
}

impl<N> RcBuilder<N> {
    fn new(node: N) -> Self {
        RcBuilder {
            inner: Arc::new(RcActivatorInner::new(node)),
            _marker: PhantomData,
            num_activators: 0,
        }
    }
}

impl<'r, N: NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r> NodeBuilder<RuntimeLoc<'r>>
    for RcBuilder<N>
{
    type Node = N;

    fn add_activator(&mut self) -> RcActivator<RuntimeNode<'r>> {
        self.inner.initial.fetch_add(1, SeqCst);

        RcActivator {
            inner: self.inner.clone(),
        }
    }

    fn finalize(&mut self, _builder: &mut RuntimeLoc<'r>) {
        self.inner.rearm();
        self.inner.decrement_pending();
    }
}

impl<'r, N: NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r> NodeBuilder<Toexec<'r>>
    for RcBuilder<N>
{
    type Node = N;

    fn add_activator(&mut self) -> RcActivator<RuntimeNode<'r>> {
        self.inner.initial.fetch_add(1, SeqCst);

        RcActivator {
            inner: self.inner.clone(),
        }
    }

    fn finalize(&mut self, _builder: &mut Toexec<'r>) {
        self.inner.rearm();
        self.inner.decrement_pending();
    }
}

impl<'a, 'r: 'a, N: NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r> NodeBorrowMut<'a, RuntimeLoc<'r>>
    for RcBuilder<N>
{
    type RefMut = MutexGuard<'a, N>;

    fn borrow_mut(&'a mut self) -> Self::RefMut {
        self.inner.handle.lock().unwrap()
    }
}

impl<'a, 'r: 'a, N: NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r> NodeBorrowMut<'a, Toexec<'r>>
    for RcBuilder<N>
{
    type RefMut = MutexGuard<'a, N>;

    fn borrow_mut(&'a mut self) -> Self::RefMut {
        self.inner.handle.lock().unwrap()
    }
}

/// The type of nodes manipulated by the parallel reusable runtime.
pub type RuntimeNode<'r> = dyn NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r;

pub type RuntimeActivator<'r> = RcActivator<RuntimeNode<'r>>;

/// A worker doing work stealing
pub struct RuntimeLoc<'r> {
    pub ready: deque::Worker<RcHandle<RuntimeNode<'r>>>,
    pub stealers: Vec<deque::Stealer<RcHandle<RuntimeNode<'r>>>>,
}

impl<'r> Scheduler for RuntimeLoc<'r> {
    type Handle = RcHandle<RuntimeNode<'r>>;

    fn schedule(&mut self, handle: Self::Handle) {
        self.ready.push(handle);
    }
}

impl<'r> Scheduler for Toexec<'r> {
    type Handle = RcHandle<RuntimeNode<'r>>;

    fn schedule(&mut self, handle: Self::Handle) {
        self.ready.push(handle);
    }
}

/// A parallel runtime for reusable graphs.
pub struct Toexec<'r> {
    pub ready: Vec<RcHandle<RuntimeNode<'r>>>,
}

impl<'r> Toexec<'r> {
    pub fn new() -> Self {
        Toexec { ready: Vec::new(),}
    }

    pub fn execute(&mut self, k: usize) {    	
        // création des listes de taches 
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
		
                scope.spawn(move || {

                    let mut runtime_loc = RuntimeLoc {
                        ready: ready_j,
                        stealers: stealers_j,
                    };
                    
                    loop {
                        match runtime_loc.ready.pop() {
                            Some(t) => t.execute_once(&mut runtime_loc),
                            None => {
                                let mut i = 0;
                                let mut tour = Arc::new(Compteur::new(0));
                                loop {
                                    match runtime_loc.stealers[i].steal() {
                                        Some(t) => {
					                        t.execute_once(&mut runtime_loc);
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
                                }
                            }
                        }
                    }
                });
            }
        });
    }
}

impl<'r> GraphSpec for RuntimeLoc<'r> {
    type Activator = RuntimeActivator<'r>;
}

impl<'r> GraphSpec for Toexec<'r> {
    type Activator = RuntimeActivator<'r>;
}

impl<'r, N: NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r> NodeSpec<N> for RuntimeLoc<'r> {
    type Builder = RcBuilder<N>;

    fn node(&self, node: N) -> Self::Builder {
        RcBuilder::new(node)
    }
}

impl<'r, N: NodeMut<RuntimeLoc<'r>> + Send + Sync + 'r> NodeSpec<N> for Toexec<'r> {
    type Builder = RcBuilder<N>;

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

impl<'r, T: Default + 'r> PortSpec<T> for Toexec<'r> {
    type Port = RcPort<Mutex<T>>;

    fn port(&self, init: T) -> Self::Port {
        RcPort::new(Mutex::new(init))
    }
}