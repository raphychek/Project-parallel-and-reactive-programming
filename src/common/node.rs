//! Common implementations for nodes.

use api::prelude::*;

/// A dummy node which panics when executed.
///
/// This can be used to create uninitialized activators when creating nodes before all their output
/// activators are available.
pub struct UninitializedNode;

impl<S> NodeOnce<S> for UninitializedNode {
    fn execute_once(self, _scheduler: &mut S) {
        panic!("Uninitialized node was executed.");
    }
}

impl<S> NodeMut<S> for UninitializedNode {
    fn execute_mut(&mut self, _scheduler: &mut S) {
        panic!("Uniniialized node was executed.");
    }
}

/// A node which bundles a task with the corresponding input and output edges.
pub struct TaskNode<I: Tuple, O: Tuple, T> {
    /// The inputs for the node.  This should be a tuple of `InputEdge` instances.
    pub inputs: I,
    /// The outputs of the node.  This should be a tuple of `OutputEdge` instances.
    pub outputs: O,
    /// The task to execute.  This should be an instance of `TaskOnce`, `TaskMut` or `Task`.
    pub task: T,
}

/// Helper structure to enforce that the underlying task can only use the node's outputs once
/// during its execution.  The implementations of `NodeMut` below wraps the underlying mutable
/// reference into an `OutputOnce` before passing the outputs to the task.
#[derive(Debug)]
pub struct OutputOnce<T>(T);

impl<'a, S, O: OutputEdgeOnce<S>> OutputEdgeOnce<S> for OutputOnce<O> {
    type Item = O::Item;

    fn send_activate_once(self, scheduler: &mut S, item: Self::Item) {
        OutputEdgeOnce::send_activate_once(self.0, scheduler, item)
    }
}

impl<'a, S, O: OutputEdgeMut<S>> OutputEdgeOnce<S> for &'a mut O {
    type Item = O::Item;

    fn send_activate_once(self, scheduler: &mut S, item: Self::Item) {
        OutputEdgeMut::send_activate_mut(self, scheduler, item)
    }
}

/// Helper structure to enforce that the underlying task can only use the node's inputs once during
/// its execution.  The implementations of `NodeMut` below wraps the underlying mutable reference
/// into an `InputOnce` before passing the inputs to the task.
#[derive(Debug)]
pub struct InputOnce<T>(T);

impl<'a, S, I: InputEdgeOnce<S>> InputEdgeOnce<S> for InputOnce<I> {
    type Item = I::Item;

    fn recv_activate_once(self, scheduler: &mut S) -> Self::Item {
        InputEdgeOnce::recv_activate_once(self.0, scheduler)
    }
}

impl<'a, S, I: InputEdgeMut<S>> InputEdgeOnce<S> for &'a mut I {
    type Item = I::Item;

    fn recv_activate_once(self, scheduler: &mut S) -> Self::Item {
        InputEdgeMut::recv_activate_mut(self, scheduler)
    }
}

// Implement the `NodeOnce` and `NodeMut` traits for `TaskNode`.  This is simply calling the
// appropriate `run` function on the underlying task with the inputs and outputs from the node,
// wrapped in `InputOnce` and `OutputOnce` structures to enforce statically that they can be only
// used once during a single task execution.
macro_rules! auto_impl_node_tuple {
    (__next impl<($($Xs:ident),* ! ) -> ()>) => {};
    (__next impl<($($Xs:ident),* ! ) -> ($O:ident $(, $Os:ident)*)>) => {
        auto_impl_node_tuple! {
            impl<(! $($Xs),*) -> ($($Os),*)>
        }
    };
    (__next impl<($($Xs:ident),* ! $I:ident $(, $Is:ident)*) -> ($($Os:ident),*)>) => {
        auto_impl_node_tuple! {
            impl<($($Xs,)* $I ! $($Is),*) -> ($($Os),*)>
        }
    };
    (impl<($($Xs:ident),* ! $($Is:ident),*) -> ($($Os:ident),*)>) => {
        impl<S, $($Is,)* $($Os,)* T: TaskOnce<($(InputOnce<$Is>,)*), ($(OutputOnce<$Os>,)*), S>>
            NodeOnce<S> for TaskNode<($($Is,)*), ($($Os,)*), T>
        {
            fn execute_once(self, scheduler: &mut S) {
                #[allow(non_snake_case)]
                let ($($Is,)*) = self.inputs;
                #[allow(non_snake_case)]
                let ($($Os,)*) = self.outputs;

                self.task.run_once(
                    scheduler,
                    ($(InputOnce($Is),)*),
                    ($(OutputOnce($Os),)*))
            }
        }

        impl<S, $($Is,)* $($Os,)* T: for<'a> TaskMut<($(InputOnce<&'a mut $Is>,)*), ($(OutputOnce<&'a mut $Os>,)*), S>>
            NodeMut<S> for TaskNode<($($Is,)*), ($($Os,)*), T>
        {
            fn execute_mut(&mut self, scheduler: &mut S) {
                #[allow(non_snake_case)]
                let ($(ref mut $Is,)*) = self.inputs;
                #[allow(non_snake_case)]
                let ($(ref mut $Os,)*) = self.outputs;

                self.task.run_mut(
                    scheduler,
                    ($(InputOnce($Is),)*),
                    ($(OutputOnce($Os),)*))
            }
        }

        auto_impl_node_tuple! {
            __next impl<($($Xs),* ! $($Is),*) -> ($($Os),*)>
        }
    };
}

auto_impl_node_tuple! {
    impl<
        (! I0, I1, I2, I3, I4, I5, I6, I7, I8, I9) ->
            (O0, O1, O2, O3, O4, O5, O6, O7, O8, O9)
    >
}
