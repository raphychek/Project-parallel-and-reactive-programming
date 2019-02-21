//! Common edge implementations.
//!
//! This module is meant to include generic implementations for edges.
//!
//! This includes a `CloneOutput` type which allows combining multiple output edges as one, cloning
//! the underlying data into each of the edges.
//!
//! It also includes macro implementations to allow considering tuples of input edges as a single
//! input edge receiving a tuple of values, and tuples of output edges as a single output edge
//! accepting a tuple of values.  This can be convenient when writing generic tasks.

use api::prelude::*;

/// An output edge which clones its output and propagates it to additional edges.
///
/// Nodes which are expected to have multiple outputs should use this structure as an output edge.
#[derive(Debug)]
pub struct CloneOutput<E> {
    outputs: Vec<E>,
}

impl<E> CloneOutput<E> {
    /// Create a new `CloneOutput` instance for a statically known type of edges.  In practice, you
    /// will probably want to use `new_box_once` or `new_box_mut` which use dynamic trait objects
    /// instead.
    pub fn new() -> Self {
        CloneOutput {
            outputs: Vec::new(),
        }
    }
}

impl<T: Clone, S: ?Sized> CloneOutput<Box<dyn OutputEdgeBox<S, Item = T>>> {
    /// Create a new `CloneOutput` instance for dynamic `OutputEdgeBox` edges.  Note that you may
    /// want to create a `Send` version of this function.
    pub fn new_box_once() -> Self {
        CloneOutput {
            outputs: Vec::new(),
        }
    }
}

impl<T: Clone, S: ?Sized> CloneOutput<Box<dyn OutputEdgeMut<S, Item = T>>> {
    /// Create a new `CloneOutput` instance for dynamic `OutputEdgeMut` reusable edges.  Note that
    /// you may want to create a `Send` version of this function.
    pub fn new_box_mut() -> Self {
        CloneOutput {
            outputs: Vec::new(),
        }
    }
}

impl<E> CloneOutput<E> {
    /// Connect an additional edge to this output.  It will be activated with a clone of the data
    /// when the `CloneOutput` is activated.
    pub fn connect(&mut self, output: E) {
        self.outputs.push(output)
    }
}

impl<S, E: OutputEdgeOnce<S>> OutputEdgeOnce<S> for CloneOutput<E>
where
    E::Item: Clone,
{
    type Item = E::Item;

    fn send_activate_once(self, scheduler: &mut S, item: Self::Item) {
        for output in self.outputs {
            output.send_activate_once(scheduler, item.clone());
        }
    }
}

impl<S, E: OutputEdgeMut<S>> OutputEdgeMut<S> for CloneOutput<E>
where
    E::Item: Clone,
{
    fn send_activate_mut(&mut self, scheduler: &mut S, item: Self::Item) {
        for output in self.outputs.iter_mut() {
            output.send_activate_mut(scheduler, item.clone());
        }
    }
}

impl<S, E: OutputEdge<S>> OutputEdge<S> for CloneOutput<E>
where
    E::Item: Clone,
{
    fn send_activate(&self, scheduler: &mut S, item: Self::Item) {
        for output in self.outputs.iter() {
            output.send_activate(scheduler, item.clone());
        }
    }
}

impl<S, E: OutputEdgeBox<S> + ?Sized> OutputEdgeOnce<S> for Box<E> {
    type Item = E::Item;

    fn send_activate_once(self, scheduler: &mut S, item: Self::Item) {
        self.send_activate_box(scheduler, item)
    }
}

impl<S, E: OutputEdgeMut<S> + ?Sized> OutputEdgeMut<S> for Box<E> {
    fn send_activate_mut(&mut self, scheduler: &mut S, item: Self::Item) {
        OutputEdgeMut::send_activate_mut(&mut **self, scheduler, item)
    }
}

impl<S, E: OutputEdge<S> + ?Sized> OutputEdge<S> for Box<E> {
    fn send_activate(&self, scheduler: &mut S, item: Self::Item) {
        OutputEdge::send_activate(&**self, scheduler, item)
    }
}

macro_rules! auto_type_item {
    (! $T:ty) => {
        type Item = $T;
    };
    ($($rest:tt)*) => {};
}

macro_rules! auto_impl_input_tuple {
    (impl<> $($Xs:ident :: $xs:ident($Selfs:ty);)* !) => {};

    (impl<$I:ident<$T:ident>, $($Is:ident<$Ts:ident>,)*>
      $($Xs:ident :: $xs:ident($Selfs:ty);)* !) => {
        auto_impl_input_tuple! {
            impl<$($Is<$Ts>,)*> ! $($Xs::$xs($Selfs);)*
        }
    };

    (impl<$($Is:ident<$Ts:ident>,)*>
      $($Xs:ident :: $xs:ident($Selfs:ty);)*
      ! $InputEdge:ident :: $recv:ident($Self:ty);
      $($rest:tt)*) => {
        impl<S, $($Ts, $Is: $InputEdge<S, Item = $Ts>,)*>
            $InputEdge<S> for ($($Is,)*)
        {
            auto_type_item!($($Xs)* ! ($($Ts,)*));

            #[allow(unused)]
            fn $recv(self: $Self, scheduler: &mut S) -> ($($Ts,)*) {
                #[allow(non_snake_case)]
                let ($($Is,)*) = self;
                ($($Is.$recv(scheduler),)*)
            }
        }

        auto_impl_input_tuple! {
            impl<$($Is<$Ts>,)*>
                $($Xs::$xs($Selfs);)*
                $InputEdge::$recv($Self);
                ! $($rest)*
        }
    };
}

auto_impl_input_tuple! {
    impl<
        R0<A0>,
        R1<A1>,
        R2<A2>,
        R3<A3>,
        R4<A4>,
        R5<A5>,
        R6<A6>,
        R7<A7>,
        R8<A8>,
        R9<A9>,
    >
        ! InputEdgeOnce::recv_activate_once(Self);
        InputEdgeMut::recv_activate_mut(&mut Self);
        InputEdge::recv_activate(&Self);
}

macro_rules! auto_impl_output_tuple {
    (impl<> $($Xs:ident :: $xs:ident($Selfs:ty);)* !) => {};

    (impl<$S:ident<$O:ident>, $($Ss:ident<$Os:ident>,)*>
      $($Xs:ident :: $xs:ident($Selfs:ty);)* !) => {
        auto_impl_output_tuple! {
            impl<$($Ss<$Os>,)*> ! $($Xs::$xs($Selfs);)*
        }
    };

    (impl<$($Ss:ident<$Os:ident>,)*>
      $($Xs:ident :: $xs:ident($Selfs:ty);)*
      ! $OutputEdge:ident :: $send:ident($Self:ty);
      $($rest:tt)*) => {
        impl<S, $($Os, $Ss: $OutputEdge<S, Item = $Os>,)*>
            $OutputEdge<S> for ($($Ss,)*)
        {
            auto_type_item!($($Xs)* ! ($($Os,)*));

            #[allow(non_snake_case, unused)]
            fn $send(self: $Self, scheduler: &mut S, ($($Os,)*): ($($Os,)*)) {
                #[allow(non_snake_case)]
                let ($($Ss,)*) = self;
                $(
                    $Ss.$send(scheduler, $Os);
                )*
            }
        }

        auto_impl_output_tuple! {
            impl<$($Ss<$Os>,)*>
                $($Xs::$xs($Selfs);)*
                $OutputEdge::$send($Self);
                ! $($rest)*
        }
    };
}

auto_impl_output_tuple! {
    impl<
        S0<O0>,
        S1<O1>,
        S2<O2>,
        S3<O3>,
        S4<O4>,
        S5<O5>,
        S6<O6>,
        S7<O7>,
        S8<O8>,
        S9<O9>,
    >
        ! OutputEdgeOnce::send_activate_once(Self);
        OutputEdgeMut::send_activate_mut(&mut Self);
        OutputEdge::send_activate(&Self);
}
