use std::{cell::RefCell, rc::Rc};

/// A shared resource wrapper that provides interior mutability.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct Shared<T> {
    pub inner: Rc<RefCell<T>>,
}

impl<T> Shared<T> {
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
    pub fn new_cyclic(value: impl FnOnce(WeakShared<T>) -> T) -> Self {
        let rc = Rc::new_cyclic(|weak| {
            RefCell::new(value(WeakShared {
                inner: weak.clone(),
            }))
        });
        Self { inner: rc }
    }

    /// Downgrades the resource to a weak reference.
    pub fn downgrade(&self) -> WeakShared<T> {
        WeakShared {
            inner: Rc::downgrade(&self.inner),
        }
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

/// A weak reference to a Shared pointer.
#[derive(Debug)]
pub struct WeakShared<T> {
    inner: std::rc::Weak<RefCell<T>>,
}

impl<T> Clone for WeakShared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> WeakShared<T> {
    pub fn upgrade(&self) -> Option<Shared<T>> {
        self.inner.upgrade().map(|rc| Shared { inner: rc })
    }
}
