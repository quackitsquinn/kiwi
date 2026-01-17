use crate::{
    component::resource::ComponentPtr,
    prelude::{ComponentReadGuard, ComponentWriteGuard},
};

/// A handle to a component.
pub struct ComponentHandle<T: 'static> {
    ptr: ComponentPtr, // haha, now only a pointer
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ComponentHandle<T> {
    pub(super) fn new(ptr: ComponentPtr) -> Self {
        assert!(ptr.is::<T>());
        Self {
            ptr,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a standalone ComponentHandle from a component.
    pub(super) fn standalone(component: T) -> Self
    where
        T: Send + Sync,
    {
        let ptr = ComponentPtr::new(component);
        Self {
            ptr,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Gets a reference to the component.
    #[deprecated = "use read() instead"]
    pub fn get(&self) -> ComponentReadGuard<T> {
        self.ptr.read()
    }

    /// Gets a mutable reference to the component.
    #[deprecated = "use write() instead"]
    #[track_caller]
    pub fn get_mut(&self) -> ComponentWriteGuard<T> {
        self.ptr.write()
    }

    /// Returns a read guard to the component.
    pub fn read(&self) -> ComponentReadGuard<T> {
        self.ptr.read()
    }

    /// Returns a write guard to the component.
    #[track_caller]
    pub fn write(&self) -> ComponentWriteGuard<T> {
        self.ptr.write()
    }
}

impl<T> Clone for ComponentHandle<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for ComponentHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentHandle")
            .field("type", &std::any::type_name::<T>())
            .finish()
    }
}
