//! Ports provides the communication mechanisms that tasks use to communicate.
//!
//! Conceptually, a port represent an area of memory which holds a single value at once (or no
//! value, when empty).  Ports can be split into two parts, a sending part and a receiving part.
//!
//! Ports are very similar to channels; however, ports assume an external synchronization mechanism
//! which ensures that data is written before being read.  Hence, while channels accept the
//! following behaviors, they are usually a logic error for ports and can lead to panic:
//!
//!  - Writing into a port that is full.
//!  - Reading from an empty port.
//!
//!  Depending on the port, the semantics in those cases can be well defined; the exact semantics
//!  of each port can vary.  However, the general contract that ports should uphold is that if
//!  sending and receiving from the port alternate in a 1-1 fashion, the last written value should
//!  be read each time.

/// A sender which can be only used once.
///
/// This is the usual way data is sent in single-use dynamic graphs.  Some implementations could be
/// made more efficient by knowing the sender won't be reused; however, this trait is mostly about
/// consistency and providing type-level information that data should not be sent twice on the same
/// port during execution.
pub trait SenderOnce {
    /// The type of items which can be sent through this sender.
    type Item;

    /// Send an item and consume the sender.
    fn send_once(self, item: Self::Item);
}

/// A sender which can be used repeatedly and may mutate local state.
///
/// This is the usual way data is sent in multiple-use graphs.  The ability to mutate local state
/// (in the sender itself, not an underlying shared port) can be taken advantage of to build debug
/// wrappers which can log information about what data is sent and when.
pub trait SenderMut: SenderOnce {
    /// When an item is sent using this method, the sender is free to mutate any local data.
    fn send_mut(&mut self, item: Self::Item);
}

/// A sender which can be used repeatedly without mutating state.
///
/// This trait is usually implemented by "raw" port types backed by some sort of interior
/// mutability and which are simply concerned with the writing of data into the port.  Smarter
/// sender types can then be built by wrapping the raw port into structures providing shared access
/// such as immutable references or reference counted pointers.
pub trait Sender: SenderMut {
    /// When an item is sent using this method, no mutation of local state is allowed.
    fn send(&self, item: Self::Item);
}

/// A receiver which can be only used once.
///
/// This is the usual way data is sent in single-use dynamic graphs.  Some implementations could be
/// made more efficient by knowing the receiver won't be reused; however, this trait is mostly
/// about consistency and providing type-level information that data should not be read twice on
/// the same port during execution.
pub trait ReceiverOnce {
    /// The type of items that are read by this receiver.
    type Item;

    /// Consume the receiver and receive an item.
    fn recv_once(self) -> Self::Item;
}

/// A receiver which can be used repeatedly and may mutate local state.
///
/// This is the most common way receivers are used.  The ability to mutate local state (in the
/// receiver itself, not an underlying shared port) can be taken advantage of to build debug
/// wrappers which can log information about what data is received and when.
pub trait ReceiverMut: ReceiverOnce {
    /// When an item is received using this method, the receiver is free to mutate any local state.
    fn recv_mut(&mut self) -> Self::Item;
}

/// A receiver which can be used repeatedly without mutating state.
///
/// This is usually implemented by "raw" port types backed by some sort of interior mutability and
/// which are simply concerned with the reading of data from the port.  Smarted receiver types can
/// then be built by wrapping the raw port into structures providing shared access such as
/// immutable references or reference counted pointers.
pub trait Receiver: ReceiverMut {
    /// When an item is received using this method, no mutation of local state is allowed.
    fn recv(&self) -> Self::Item;
}

/// A port, which can be separated into a sending and receiving part.  This is provided as a helper
/// trait for building higher-level graph building APIs.
pub trait Port {
    /// The sending part of the port.
    ///
    /// This will usually implement some of the `Sender` family of traits, but could also be a
    /// singleton type such as `()` for ports which use an auxiliary mechanism to read data (for
    /// instance, a constant port, or a port which reads values from a file).
    type Sender;

    /// The receiving part of the port
    ///
    /// This will usually implement some of the `Receiver` family of traits, but could also be a
    /// singleton type such as `()` to implement a port which uses an auxiliary mechanism to send
    /// data (for instance, a port which writes values to a file, or a logging port).
    type Receiver;

    /// Separate the port into its sending and receiving parts.  As noted above, depending on the
    /// port, the sending or receiving part may not be present.
    fn split(self) -> (Self::Sender, Self::Receiver);
}
