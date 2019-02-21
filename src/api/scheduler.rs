//! The scheduling API

pub trait Scheduler {
    type Handle;

    fn schedule(&mut self, handle: Self::Handle);
}
