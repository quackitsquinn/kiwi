use std::{cell::RefCell, ops::Deref, rc::Rc};

/// A shared resource wrapper that provides interior mutability.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct Resource<T> {
    pub inner: Rc<RefCell<T>>,
}

impl<T> Resource<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(value)),
        }
    }

    /// Borrows the resource immutably.
    pub fn get(&self) -> std::cell::Ref<'_, T> {
        self.inner.borrow()
    }

    /// Borrows the resource mutably.
    pub fn get_mut(&self) -> std::cell::RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    /// Creates a new cyclic Resource.
    ///
    /// This was primarily added for GameState to hold a Weak reference to itself.
    pub fn new_cyclic(value: impl FnOnce(WeakResource<T>) -> T) -> Self {
        let rc = Rc::new_cyclic(|weak| {
            RefCell::new(value(WeakResource {
                inner: weak.clone(),
            }))
        });
        Self { inner: rc }
    }

    /// Downgrades the resource to a weak reference.
    pub fn downgrade(&self) -> WeakResource<T> {
        WeakResource {
            inner: Rc::downgrade(&self.inner),
        }
    }
}

impl<T> From<T> for Resource<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

/// A weak reference to a Resource.
#[derive(Debug)]
pub struct WeakResource<T> {
    inner: std::rc::Weak<RefCell<T>>,
}

impl<T> Clone for WeakResource<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> WeakResource<T> {
    pub fn upgrade(&self) -> Option<Resource<T>> {
        self.inner.upgrade().map(|rc| Resource { inner: rc })
    }
}

/// A resource wrapper for a immutable resource.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct ImmutableResource<T> {
    pub inner: std::rc::Rc<T>,
}

impl<T> ImmutableResource<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: std::rc::Rc::new(value),
        }
    }

    pub fn downgrade(&self) -> WeakImmutableResource<T> {
        WeakImmutableResource {
            inner: std::rc::Rc::downgrade(&self.inner),
        }
    }

    pub fn new_cyclic(value: impl FnOnce(WeakImmutableResource<T>) -> T) -> Self {
        let rc = std::rc::Rc::new_cyclic(|weak| {
            value(WeakImmutableResource {
                inner: weak.clone(),
            })
        });
        Self { inner: rc }
    }
}

impl<T> From<T> for ImmutableResource<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Clone for ImmutableResource<T> {
    fn clone(&self) -> Self {
        Self {
            inner: std::rc::Rc::clone(&self.inner),
        }
    }
}

// ImmutableResource has a *really* nice Deref impl.
impl<T> Deref for ImmutableResource<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub struct WeakImmutableResource<T> {
    inner: std::rc::Weak<T>,
}

impl<T> WeakImmutableResource<T> {
    pub fn upgrade(&self) -> Option<ImmutableResource<T>> {
        self.inner
            .upgrade()
            .map(|rc| ImmutableResource { inner: rc })
    }
}

impl<T> Clone for WeakImmutableResource<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
