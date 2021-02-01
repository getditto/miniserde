use ::std::ptr;

/// A `Box` that may be aliased after creation and before destruction.
#[repr(transparent)]
pub struct AliasedBox<T: ?Sized>(ptr::NonNull<T>);

impl<T: ?Sized> From<Box<T>> for AliasedBox<T> {
    fn from(p: Box<T>) -> AliasedBox<T> {
        Self(Box::leak(p).into())
    }
}

impl<T> AliasedBox<T> {
    pub fn new(value: T) -> AliasedBox<T> {
        Self::from(Box::new(value))
    }
}

impl<T: ?Sized> Drop for AliasedBox<T> {
    fn drop(self: &'_ mut Self) {
        unsafe { drop::<Box<T>>(Box::from_raw(self.0.as_ptr())) }
    }
}

impl<T: ?Sized> AliasedBox<T> {
    pub fn ptr(self: &'_ AliasedBox<T>) -> *mut T {
        self.0.as_ptr()
    }

    pub fn assume_unique(self: AliasedBox<T>) -> Box<T> {
        // Disable drop / relinquish ownership.
        let this = ::core::mem::ManuallyDrop::new(self);
        // Ownership can now be transfered.
        unsafe { Box::from_raw(this.0.as_ptr()) }
    }
}
