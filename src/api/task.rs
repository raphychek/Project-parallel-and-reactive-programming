//! Tasks API
//!
//! A task represents a single unit of computation.  The `Task` family of traits are provided with
//! a scheduler, input ports, and output ports.
//!
//! Note that there are no trait bounds on the arguments.  This is due to variadic generics not
//! being available in Rust (see the documentation for the `Tuple` marker trait).

use super::marker::Tuple;

/// A trait for tasks which can be run only once.
///
/// Trait implementors should add constraints that the `I` and `O` types are tuples of
/// `InputSenderOnce` and `OutputSenderOnce` instances, respectively.
pub trait TaskOnce<I: Tuple, O: Tuple, S> {
    fn run_once(self, scheduler: &mut S, inputs: I, outputs: O);
}

/// A trait for tasks which can be run multiple times, but not concurrently.
///
/// Trait implementors should add constraints that the `I` and `O` types are tuples of
/// `InputSenderOnce` and `OutputSenderOnce` instances, respectively.
pub trait TaskMut<I: Tuple, O: Tuple, S> {
    fn run_mut(&mut self, scheduler: &mut S, inputs: I, outputs: O);
}

/// A trait for tasks which can be rund multiple times concurrently.
///
/// Trait implementors should add constraints that the `I` and `O` types are tuples of
/// `InputSenderOnce` and `OutputSenderOnce` instances, respectively.
pub trait Task<I: Tuple, O: Tuple, S> {
    fn run(&self, scheduler: &mut S, inputs: I, outputs: O);
}
