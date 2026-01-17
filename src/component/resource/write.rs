use std::{
    ops::{Deref, DerefMut},
    panic::Location,
    sync::atomic::Ordering,
    thread,
};

use crate::component::resource::{ComponentPtr, LockState, check_deadlock};

pub struct ComponentWriteGuard<'a, T: 'static> {
    inner: ComponentPtr,
    phantom: std::marker::PhantomData<&'a mut T>,
}

impl<'a, T: 'static> ComponentWriteGuard<'a, T> {
    /// Creates a new ComponentWriteGuard.
    ///
    /// # Safety
    ///
    /// inner must represent a valid component of type T.
    pub(crate) unsafe fn lock(inner: ComponentPtr, location: &'static Location<'static>) -> Self {
        let inner_ref = inner.get_ref();
        let this = thread::current().id().as_u64().get();

        if inner_ref.flags.load(Ordering::Relaxed) & !LockState::IS_INIT.bits() != 0 {
            panic!("Attempted to write uninitialized component");
        }

        let mut is_first = true;
        // wait until we can acquire the write lock
        while let Err(v) =
            inner_ref
                .state
                .compare_exchange(0, -1, Ordering::Acquire, Ordering::Relaxed)
        {
            if v == -1 && is_first {
                // writer is held, check for deadlock
                check_deadlock(inner_ref, "write");
            }
            is_first = false;
            thread::yield_now();
        }

        // we have the write lock, set the writer thread id and location
        inner_ref.writer.0.store(this, Ordering::Relaxed);
        inner_ref
            .writer
            .1
            .store(location as *const _ as *mut _, Ordering::Relaxed);

        Self {
            inner,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Drop for ComponentWriteGuard<'_, T> {
    fn drop(&mut self) {
        let inner_ref = self.inner.get_ref();

        // clear the writer thread id and location
        inner_ref.writer.0.store(0, Ordering::Relaxed);
        inner_ref
            .writer
            .1
            .store(std::ptr::null_mut(), Ordering::Relaxed);

        // release the write lock
        inner_ref.state.store(0, Ordering::Release);
    }
}

impl<T> Deref for ComponentWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Safety is guaranteed by the constructor.
        unsafe { &*(self.inner.inner_ref() as *const dyn std::any::Any as *const T) }
    }
}

impl<T> DerefMut for ComponentWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Safety is guaranteed by the constructor.
        unsafe { &mut *(self.inner.inner_mut() as *mut dyn std::any::Any as *mut T) }
    }
}

#[cfg(test)]
mod tests {
    use std::{panic::Location, thread};

    use crate::component::resource::{
        ComponentPtr, read::ComponentReadGuard, write::ComponentWriteGuard,
    };

    #[test]
    fn test_component_write_guard() {
        let ptr = ComponentPtr::new(42u32);
        {
            let mut guard =
                unsafe { ComponentWriteGuard::<u32>::lock(ptr.clone(), Location::caller()) };
            assert_eq!(*guard, 42u32);
            *guard = 100u32;
            assert_eq!(*guard, 100u32);
        }
        let inner_ref = ptr.get_ref();
        assert_eq!(
            inner_ref.state.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        let guard = unsafe { ComponentWriteGuard::<u32>::lock(ptr.clone(), Location::caller()) };
        assert_eq!(*guard, 100u32);
    }

    #[test]
    #[should_panic(
        expected = "Deadlock detected: thread attempted to acquire write lock while holding write lock"
    )]
    fn test_component_write_deadlock_protection() {
        let ptr = ComponentPtr::new(42u32);
        let _guard = unsafe { ComponentWriteGuard::<u32>::lock(ptr.clone(), Location::caller()) };
        // Attempting to acquire another write lock should deadlock

        let _guard2 = unsafe { ComponentWriteGuard::<u32>::lock(ptr.clone(), Location::caller()) };
    }

    #[test]
    fn test_component_write_guard_heavy_multithread() {
        let ptr = ComponentPtr::new(0u32);

        let mut handles = vec![];

        for _ in 0..10 {
            let ptr_clone = ptr.clone();
            let handle = std::thread::spawn(move || {
                let thread_id = thread::current().id().as_u64().get();
                for i in 0..10000 {
                    let mut guard = unsafe {
                        ComponentWriteGuard::<u32>::lock(ptr_clone.clone(), Location::caller())
                    };

                    assert_eq!(*guard, 0u32);
                    let val = thread_id * i;
                    *guard = val as u32;
                    assert_eq!(*guard, val as u32);
                    *guard = 0u32;
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
