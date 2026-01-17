use std::{ptr::NonNull, sync::atomic::Ordering};

use crate::component::resource::{ComponentInner, ComponentPtr};

/// A weak pointer to a component.
#[derive(Debug)]
pub struct WeakComponentPtr {
    pub(super) data: NonNull<ComponentInner>,
}

impl WeakComponentPtr {
    /// Creates a new WeakComponentPtr from a ComponentPtr.
    pub(super) unsafe fn new(data: NonNull<ComponentInner>) -> Self {
        Self { data }
    }

    /// Upgrades the weak pointer to a strong pointer, if the component is still alive.
    pub fn upgrade(&self) -> Option<ComponentPtr> {
        let inner = unsafe { self.data.as_ref() };
        let strong_count = inner.strong.load(Ordering::Relaxed);
        if strong_count == 0 {
            return None;
        }
        inner.strong.fetch_add(1, Ordering::Relaxed);
        Some(ComponentPtr { data: self.data })
    }
}

impl Drop for WeakComponentPtr {
    fn drop(&mut self) {
        let inner = unsafe { self.data.as_ref() };
        if inner.weak.fetch_sub(1, Ordering::Release) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            unsafe {
                std::alloc::dealloc(self.data.as_ptr() as *mut u8, inner.layout.0);
            }
        }
    }
}
