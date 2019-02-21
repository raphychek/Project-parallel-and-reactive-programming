//! Common task implementations.
//!
//! This module is meant to include generic implementation for tasks.

use api::prelude::*;

/// A wrapper for converting a strict function into a task.
///
/// This can be used to build a task compatible with all types of input and output edges from a
/// strict function.  From instance you can create a 3-way addition task by doing:
///
/// ```rust,ignore
/// StrictTask::new(
///     |x, y, z| (x + y + z,))
/// ```
///
/// Note that you must return a tuple of outputs (1-element tuple in this case) due to limitations
/// of the Rust type system (otherwise it would fail when converting into a node).  This is
/// enforced by the implementations below with a `Tuple` bound on the output which should generate
/// understandable error messages.
pub struct StrictTask<F> {
    inner: F,
}

impl<F> StrictTask<F> {
    /// Create a new task encapsulating a strict function.
    ///
    /// Note that the underlying function `F` must return a tuple of output values.
    pub fn new(inner: F) -> StrictTask<F> {
        StrictTask { inner }
    }
}

// Macro implementation of of the Task family of trait for StrictTask with functions of multiple
// arguments.
macro_rules! auto_impl_strict_task_tuple {
    (impl<> { $($Xs:ident :: $xs:ident($Selfs:ty) for $Fs:ident,)* ! }) => {};
    (impl<$I:ident, $($Is:ident,)*> {
        $($Xs:ident :: $xs:ident($Selfs:ty) for $Fs:ident,)* !
     }) => {
        auto_impl_strict_task_tuple! {
            impl<$($Is,)*> { ! $($Xs::$xs($Selfs) for $Fs,)* }
        }
    };
    (impl<$($Is:ident,)*> {
         $($Xs:ident :: $xs:ident($Selfs:ty) for $Fs:ident,)*
         ! $Task:ident :: $execute:ident($Self:ty) for $Fn:ident,
         $($rest:tt)*
     }) => {
        impl<S, $($Is: InputEdgeOnce<S>,)* O: Tuple + OutputEdgeOnce<S>, F: $Fn($($Is::Item,)*) -> O::Item>
            $Task<($($Is,)*), O, S> for StrictTask<F>
        {
            fn $execute(self: $Self, scheduler: &mut S, inputs: ($($Is,)*), outputs: O) {
                #[allow(non_snake_case)]
                let ($($Is,)*) = inputs;
                #[allow(non_snake_case)]
                let ($($Is,)*) = ($($Is.recv_activate_once(scheduler),)*);
                outputs
                    .send_activate_once(scheduler, (self.inner)($($Is,)*));
            }
        }

        auto_impl_strict_task_tuple! {
            impl<$($Is,)*> {
                $($Xs::$xs($Selfs) for $Fs,)*
                $Task::$execute($Self) for $Fn,
                ! $($rest)*
            }
        }
    };
}

auto_impl_strict_task_tuple! {
    impl<
        R0,
        R1,
        R2,
        R3,
        R4,
        R5,
        R6,
        R7,
        R8,
        R9,
    > {
        ! TaskOnce::run_once(Self) for FnOnce,
        TaskMut::run_mut(&mut Self) for FnMut,
        Task::run(&Self) for Fn,
    }
}
