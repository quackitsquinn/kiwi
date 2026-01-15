use std::{sync::atomic::Ordering, thread};

use crate::component::resource::{ComponentInner, ComponentPtr, check_deadlock};

/// A guard that provides read access to a component.
pub struct ComponentReadGuard<'a, T: 'static> {
    inner: ComponentPtr,
    phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T: 'static> ComponentReadGuard<'a, T> {
    /// Creates a new ComponentReadGuard.
    ///
    /// # Safety
    ///
    /// inner must represent a valid component of type T.
    pub(crate) unsafe fn lock(inner: ComponentPtr) -> Self {
        let inner_ref = inner.get_ref();

        let mut is_first = true;
        while inner_ref
            .state
            .fetch_update(Ordering::Release, Ordering::Acquire, |v| {
                if v == -1 {
                    // Since a deadlock indicates a frame higher up in the stack is holding the write lock,
                    // we can check for it here to provide a better error message.
                    // If we are deadlocked we will know right away, so we only need to check once.
                    if is_first {
                        check_deadlock(&inner_ref, "read");
                    }
                    is_first = false;
                    return None;
                }
                Some(v + 1)
            })
            .is_err()
        {
            thread::yield_now();
        }

        Self {
            inner,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, T: 'static> std::ops::Deref for ComponentReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Safety is guaranteed by the constructor.
        unsafe { &*(self.inner.inner_ref() as *const dyn std::any::Any as *const T) }
    }
}

impl<'a, T> Drop for ComponentReadGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.get_ref().state.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {

    use std::panic;

    use crate::component::resource::{ComponentPtr, read::ComponentReadGuard};

    #[test]
    fn test_component_read_guard() {
        let ptr = ComponentPtr::new(42u32);
        let guard = unsafe { ComponentReadGuard::<u32>::lock(ptr.clone()) };
        assert_eq!(*guard, 42u32);
        let inner_ref = ptr.get_ref();
        assert_eq!(
            inner_ref.state.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[test]
    fn test_component_read_guard_drop() {
        let ptr = ComponentPtr::new(42u32);
        {
            let guard = unsafe { ComponentReadGuard::<u32>::lock(ptr.clone()) };
            assert_eq!(*guard, 42u32);
        }
        let inner_ref = ptr.get_ref();
        assert_eq!(
            inner_ref.state.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_heavy_multithread() {
        let ptr = ComponentPtr::new(100u32);

        let mut handles = vec![];

        for _ in 0..10 {
            let ptr_clone = ptr.clone();
            let handle = std::thread::spawn(move || {
                for _ in 0..10000 {
                    let guard = unsafe { ComponentReadGuard::<u32>::lock(ptr_clone.clone()) };
                    assert_eq!(*guard, 100u32);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let inner_ref = ptr.get_ref();
        assert_eq!(
            inner_ref.state.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(
            inner_ref.strong.load(std::sync::atomic::Ordering::Relaxed),
            1
        )
    }
}
