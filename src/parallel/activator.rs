//! Activator implementations for use with sequential runtimes.
//!
//! This implements the activator traits on reference counted activators in order to allow sharing
//! activators for nodes whose inputs can come from multiple source nodes.

use api::prelude::*;

//use std::rc::Rc;
use std::sync::Arc;

impl<S, A: Activator<S>> ActivatorOnce<S> for Arc<A> {
    fn activate_once(self, scheduler: &mut S) {
        Activator::activate(&self, scheduler)
    }
}

impl<S, A: Activator<S>> ActivatorMut<S> for Arc<A> {
    fn activate_mut(&mut self, scheduler: &mut S) {
        Activator::activate(self, scheduler)
    }
}

impl<S, A: Activator<S>> Activator<S> for Arc<A> {
    fn activate(&self, scheduler: &mut S) {
        Activator::activate(&**self, scheduler)
    }
}
