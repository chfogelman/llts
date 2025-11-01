use core::ops::{Deref, DerefMut};
use std::fmt::{Debug, Display};

#[repr(align(64))]
#[derive(Clone, Copy, Default)]
pub struct CacheLineAligned<T>(pub T);

impl<T> Deref for CacheLineAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for CacheLineAligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Debug> Debug for CacheLineAligned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Display> Display for CacheLineAligned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
