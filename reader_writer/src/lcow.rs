use std::{borrow::Borrow, fmt, ops::Deref};

/// A lenient Cow
///
/// Similar to std::borrow::Cow, with an optional ToOwned/Clone bound on T.
pub enum LCow<'r, T> {
    Borrowed(&'r T),
    Owned(T),
}

impl<T> Clone for LCow<'_, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match *self {
            LCow::Borrowed(t) => LCow::Borrowed(t),
            LCow::Owned(ref t) => LCow::Owned(t.clone()),
        }
    }
}

impl<T> LCow<'_, T>
where
    T: Clone,
{
    pub fn into_owned(self) -> T {
        match self {
            LCow::Borrowed(t) => t.clone(),
            LCow::Owned(t) => t,
        }
    }
}

impl<T> Deref for LCow<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match *self {
            LCow::Borrowed(t) => t,
            LCow::Owned(ref t) => t,
        }
    }
}

impl<T> Borrow<T> for LCow<'_, T> {
    fn borrow(&self) -> &T {
        match *self {
            LCow::Borrowed(r) => r,
            LCow::Owned(ref t) => t,
        }
    }
}

impl<T> fmt::Debug for LCow<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <T as fmt::Debug>::fmt(self, f)
    }
}

// TODO: Other std traits?
