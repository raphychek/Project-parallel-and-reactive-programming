//! Common port implementations and extensions.

use api::prelude::*;

/// A trait containing extensions for the `Receiver` family of traits.  It provides convenience
/// methods to facilitate usage of types implementing those traits.
pub trait ReceiverExt: Sized {
    /// Convert a receiver into a pure data input edge.  The input edge doesn't have a control
    /// component and receiving data through it will never activate another node.
    fn as_data_input(self) -> DataInput<Self>;
}

impl<T: ReceiverOnce> ReceiverExt for T {
    fn as_data_input(self) -> DataInput<Self> {
        DataInput { receiver: self }
    }
}

/// A newtype wrapper converting a receiver into a pure data edge with no control component.
///
/// We use a newtype wrapper for symmetry with the `DataOutput` structure and in order to
/// explicitely indicate that the receiver is meant to be used as a pure data edge.
///
/// See also the `as_data_input` method from the `ReceiverExt` trait.
#[derive(Debug)]
pub struct DataInput<T> {
    receiver: T,
}

impl<S, T: ReceiverOnce> InputEdgeOnce<S> for DataInput<T> {
    type Item = T::Item;

    fn recv_activate_once(self, _: &mut S) -> Self::Item {
        self.receiver.recv_once()
    }
}

impl<S, T: ReceiverMut> InputEdgeMut<S> for DataInput<T> {
    fn recv_activate_mut(&mut self, _: &mut S) -> Self::Item {
        self.receiver.recv_mut()
    }
}

impl<S, T: Receiver> InputEdge<S> for DataInput<T> {
    fn recv_activate(&self, _: &mut S) -> Self::Item {
        self.receiver.recv()
    }
}

/// A trait containing extension for the `Sender` family of traits.  It provides convenience
/// methods to facilitate usage of types implementing those traits.
pub trait SenderExt: Sized {
    /// Bundles a sender with an activator for the corresponding node into an output edge.
    ///
    /// # Note
    ///
    /// That there is no constraint on the `A` type.  We should use a `A: ActivatorOnce<Sc>` bound
    /// where `Sc` is a scheduler type, but since activators are usually compatible with a wide
    /// variety of schedulers this would make for a lot of "missing type annotations" ambiguity
    /// errors.  Instead, we allow `with_activator` to be called with any type, and delegate the
    /// checks that it was actually an activator type to when the `NodeInput` edge gets used.
    fn with_activator<A>(self, activator: A) -> NodeInput<A, Self>;

    /// Convert a sender into a pure data output edge.  The output edge doesn't have a control
    /// component and sending data through it will never activate another node.
    ///
    /// This can be useful in cases where the sender should communicate with external resources
    /// (for instance, writing data to a file, a debug stream, or a remote database).  In addition,
    /// it can be used to implement memories in a static graph: at each execution, the node can
    /// store a value in the sender which will be read during the next execution.  Having a control
    /// component in the edge in this case would prevent the node from ever running.
    fn as_data_output(self) -> DataOutput<Self>;
}

impl<T: SenderOnce> SenderExt for T {
    fn with_activator<A>(self, activator: A) -> NodeInput<A, Self> {
        NodeInput {
            activator,
            sender: self,
        }
    }

    fn as_data_output(self) -> DataOutput<Self> {
        DataOutput { sender: self }
    }
}

/// A wrapper converting an activator and sender into an output edge.  When activated, the edge
/// will first send data into the sender, then activate the activator.
///
/// The expectation (from which `NodeInput` takes its name) is that the sender should write into a
/// port that will be read by the node activated by the activator; however this is not guaranteed
/// in any way.  In particular this may not be true at all for buggy code.
///
/// See also the `with_activator` method from the `SenderExt` trait.
#[derive(Debug, Clone)]
pub struct NodeInput<A, I> {
    pub activator: A,
    pub sender: I,
}

impl<S, A: ActivatorOnce<S>, I: SenderOnce> OutputEdgeOnce<S> for NodeInput<A, I> {
    type Item = I::Item;

    fn send_activate_once(self, scheduler: &mut S, item: Self::Item) {
        self.sender.send_once(item);
        self.activator.activate_once(scheduler);
    }
}

impl<S, A: ActivatorMut<S>, I: SenderMut> OutputEdgeMut<S> for NodeInput<A, I> {
    fn send_activate_mut(&mut self, scheduler: &mut S, item: Self::Item) {
        self.sender.send_mut(item);
        self.activator.activate_mut(scheduler);
    }
}

impl<S, A: Activator<S>, I: Sender> OutputEdge<S> for NodeInput<A, I> {
    fn send_activate(&self, scheduler: &mut S, item: Self::Item) {
        self.sender.send(item);
        self.activator.activate(scheduler);
    }
}

/// A newtype wrapper converting a sender into a pure data edge (with no control component).
///
/// We use a newtype wrapper instead of directly implementing the `OutputEdge` family of traits in
/// order to avoid mistakes when the programmer forgets to add an activator to the sender in an
/// edge context.
///
/// See also the `as_data_output` method from the `SenderExt` trait.
#[derive(Debug)]
pub struct DataOutput<T> {
    sender: T,
}

impl<S, T: SenderOnce> OutputEdgeOnce<S> for DataOutput<T> {
    type Item = T::Item;

    fn send_activate_once(self, _: &mut S, item: Self::Item) {
        self.sender.send_once(item);
    }
}

impl<S, T: SenderMut> OutputEdgeMut<S> for DataOutput<T> {
    fn send_activate_mut(&mut self, _: &mut S, item: Self::Item) {
        self.sender.send_mut(item);
    }
}

impl<S, T: Sender> OutputEdge<S> for DataOutput<T> {
    fn send_activate(&self, _: &mut S, item: Self::Item) {
        self.sender.send(item);
    }
}

/// The sending part of a `RefPort`.  Wraps a `Sender` inside a reference and expose the sending
/// methods.
#[derive(Debug, Clone)]
pub struct RefSender<'a, T: Sender + 'a>(&'a T);

impl<'a, T: Sender + 'a> SenderOnce for RefSender<'a, T> {
    type Item = T::Item;

    fn send_once(self, item: Self::Item) {
        Sender::send(&self, item)
    }
}

impl<'a, T: Sender + 'a> SenderMut for RefSender<'a, T> {
    fn send_mut(&mut self, item: Self::Item) {
        Sender::send(self, item)
    }
}

impl<'a, T: Sender + 'a> Sender for RefSender<'a, T> {
    fn send(&self, item: Self::Item) {
        Sender::send(&*self.0, item)
    }
}

/// The receiving part of a `RefPort`.  Wraps a `Receiver` inside a reference and expose the
/// receiving methods.
#[derive(Debug, Clone)]
pub struct RefReceiver<'a, T: 'a>(&'a T);

impl<'a, T: Receiver + 'a> ReceiverOnce for RefReceiver<'a, T> {
    type Item = T::Item;

    fn recv_once(self) -> Self::Item {
        Receiver::recv(&self)
    }
}

impl<'a, T: Receiver + 'a> ReceiverMut for RefReceiver<'a, T> {
    fn recv_mut(&mut self) -> Self::Item {
        Receiver::recv(self)
    }
}

impl<'a, T: Receiver + 'a> Receiver for RefReceiver<'a, T> {
    fn recv(&self) -> Self::Item {
        Receiver::recv(&*self.0)
    }
}

/// A port based on an pre-allocated area of memory.
#[derive(Debug)]
pub struct RefPort<'a, T: Sender + Receiver + 'a>(&'a mut T);

impl<'a, T: Sender + Receiver + 'a> RefPort<'a, T> {
    /// Create a new port from an underlying memory area.
    pub fn new(initial: &'a mut T) -> Self {
        RefPort(initial)
    }
}

impl<'a, T: Sender + Receiver + 'a> Port for RefPort<'a, T> {
    type Sender = RefSender<'a, T>;
    type Receiver = RefReceiver<'a, T>;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        (RefSender(self.0), RefReceiver(self.0))
    }
}
