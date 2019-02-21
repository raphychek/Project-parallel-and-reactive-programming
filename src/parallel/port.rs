//! Port implementations for use with sequential runtimes.
//!
//! This includes implementations of the `Sender` and `Receiver` traits for Rust's `Cell` type, as
//! well as a `Rc`-based implementation of a sequential reference counted port.

use api::prelude::*;
//use std::cell::Cell;
//use std::rc::Rc;
use std::sync::{Arc,Mutex};

/*
impl<T> SenderOnce for Cell<T> {
    type Item = T;

    fn send_once(self, item: Self::Item) {
        Sender::send(&self, item);
    }
}

impl<T> SenderMut for Cell<T> {
    fn send_mut(&mut self, item: Self::Item) {
        Sender::send(self, item);
    }
}

impl<T> Sender for Cell<T> {
    fn send(&self, item: Self::Item) {
        self.set(item);
    }
}
*/

impl<T> SenderOnce for Mutex<T> {
    type Item = T;

    fn send_once(self, item: Self::Item) {
        Sender::send(&self, item);
    }
}

impl<T> SenderMut for Mutex<T> {
    fn send_mut(&mut self, item: Self::Item) {
        Sender::send(self, item);
    }
}

impl<T> Sender for Mutex<T> {
    fn send(&self, item: Self::Item) {
        let mut ctnt = self.lock().unwrap();
        *ctnt = item;
    }
}

/*
impl<T> ReceiverOnce for Cell<T> {
    type Item = T;

    fn recv_once(self) -> Self::Item {
        self.into_inner()
    }
}

impl<T: Default> ReceiverMut for Cell<T> {
    fn recv_mut(&mut self) -> Self::Item {
        Receiver::recv(self)
    }
}

impl<T: Default> Receiver for Cell<T> {
    fn recv(&self) -> Self::Item {
        self.take()
    }
}
*/


impl<T> ReceiverOnce for Mutex<T> {
    type Item = T;

    fn recv_once(self) -> Self::Item {
        self.into_inner().unwrap()
    }
}

impl<T: Default> ReceiverMut for Mutex<T> {
    fn recv_mut(&mut self) -> Self::Item {
        Receiver::recv(self)
    }
}

impl<T: Default> Receiver for Mutex<T> {
    fn recv(&self) -> Self::Item {
        std::mem::replace(&mut *(self.lock().unwrap()) , Default::default() )
    }
}

/// The sending part of a `RcPort`.  Wraps a `Sender` inside a reference counter pointer and expose
/// the sending methods.
///
/// The `RcSender` implements the whole family of `Sender` traits and passes on the data to the
/// underlying sender.
#[derive(Debug)]
pub struct RcSender<T: Sender>(Arc<T>);

impl<T: Sender> Clone for RcSender<T> {
    fn clone(&self) -> Self {
        RcSender(self.0.clone())
    }
}

impl<T: Sender> SenderOnce for RcSender<T> {
    type Item = T::Item;

    fn send_once(self, item: Self::Item) {
        Sender::send(&self, item)
    }
}

impl<T: Sender> SenderMut for RcSender<T> {
    fn send_mut(&mut self, item: Self::Item) {
        Sender::send(self, item)
    }
}

impl<T: Sender> Sender for RcSender<T> {
    fn send(&self, item: Self::Item) {
        Sender::send(&*self.0, item)
    }
}

/// The receiving part of a `RcPort`.  Wraps a `Receiver` inside a reference counter pointer and
/// expose the receiving methods.
///
/// The `RcReceiver` implements the whole family of `Receiver` trants and gets the data from the
/// underlying receiver.
#[derive(Debug, Clone)]
pub struct RcReceiver<T>(Arc<T>);

impl<T: Receiver> ReceiverOnce for RcReceiver<T> {
    type Item = T::Item;

    fn recv_once(self) -> Self::Item {
        Receiver::recv(&self)
    }
}

impl<T: Receiver> ReceiverMut for RcReceiver<T> {
    fn recv_mut(&mut self) -> Self::Item {
        Receiver::recv(self)
    }
}

impl<T: Receiver> Receiver for RcReceiver<T> {
    fn recv(&self) -> Self::Item {
        Receiver::recv(&*self.0)
    }
}

/// A reference counted port.
#[derive(Debug)]
pub struct RcPort<T: Sender + Receiver>(T);

impl<T: Sender + Receiver> RcPort<T> {
    /// Create a new `RcPort` from an underlying data slot, such as a cell.
    pub fn new(initial: T) -> Self {
        RcPort(initial)
    }
}

impl<T: Sender + Receiver> Port for RcPort<T> {
    type Sender = RcSender<T>;
    type Receiver = RcReceiver<T>;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        let sender = RcSender(Arc::new(self.0));
        let receiver = RcReceiver(sender.0.clone());
        (sender, receiver)
    }
}
