/// A marker trait to indicate that a type must be a tuple.  This is used to circumvent one of
/// Rust's limitations in that there is no variadic generics, but we want to be able to use tasks
/// with variable number of inputs and outputs.
///
/// Hence, we usually do not put restrictions on the implementers of traits, and implement the
/// relevant traits using macros for tuples up to 10 elements; however, this makes for a poor user
/// experience because we can end up with arcane error messages.  One common cause for those errors
/// is to forget to wrap values in a single element tuple: the `Tuple` trait serves as a marker
/// trait to indicate that a value *must* be a tuple, and allows to give an error message
/// indicating that a tuple argument was expected.
pub trait Tuple {}

// Recursively implement the `Tuple` trait for tuples up to 10 elements. If needed, you can
// implement the trait for larger tuples by adding extra types in the `auto_impl_tuple!` call
// below.
macro_rules! auto_impl_tuple {
    (impl<>) => {
        impl Tuple for () {}
    };
    (impl<$T:ident $(, $Ts:ident)*>) => {
        impl<$T, $($Ts),*> Tuple for ($T, $($Ts),*) {}

        auto_impl_tuple!{
            impl<$($Ts),*>
        }
    };
}

auto_impl_tuple!(impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9>);
