use std::{
    alloc::Layout,
    any::Any,
    fmt,
    mem::MaybeUninit,
    panic::Location,
    ptr::NonNull,
    sync::atomic::{
        AtomicBool, AtomicIsize, AtomicPtr, AtomicU8, AtomicU64, AtomicUsize, Ordering,
    },
    thread,
};

use bitflags::bitflags;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod read;
mod weak;
pub mod write;

pub use weak::WeakComponentPtr;

use crate::{component::resource::read::ComponentReadGuard, prelude::ComponentWriteGuard};

/// Internal representation of a component.
/// This is modeled closely after specifically `Arc`, but with internal read/write locking that was designed by me.
///
pub struct ComponentPtr {
    data: NonNull<ComponentInner>,
}

impl ComponentPtr {
    /// Creates a new ComponentPtr wrapping the given component.
    pub(crate) fn new<T: Send + Sync + 'static>(inner: T) -> Self {
        let (layout, offset) = create_component_inner_layout::<T>();

        let raw_ptr = unsafe { std::alloc::alloc(layout) };
        if raw_ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }

        let component_ptr = unsafe { raw_ptr.add(offset) as *mut T };
        unsafe {
            component_ptr.write(inner);
        }

        let inner_ptr = raw_ptr as *mut ComponentInner;
        let component_trait_ptr: *mut (dyn Any + Send + Sync) = component_ptr;

        unsafe {
            inner_ptr.write(ComponentInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(1),
                state: AtomicIsize::new(0),
                flags: AtomicU8::new(LockState::IS_INIT.bits()),
                writer: (AtomicU64::new(0), AtomicPtr::new(std::ptr::null_mut())),
                component: Some(NonNull::new_unchecked(component_trait_ptr)),
                layout: (layout, offset),
                type_name: std::any::type_name::<T>(),
            })
        };

        Self {
            data: unsafe { NonNull::new_unchecked(inner_ptr) },
        }
    }

    /// Creates a new uninitialized ComponentPtr for the given type T.
    /// The caller is responsible for initializing the component before use.
    pub(crate) fn uninitialized<T: Send + Sync + 'static>() -> Self {
        let (layout, offset) = create_component_inner_layout::<T>();

        let raw_ptr = unsafe { std::alloc::alloc(layout) };
        if raw_ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }

        let inner_ptr = raw_ptr as *mut ComponentInner;

        unsafe {
            inner_ptr.write(ComponentInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(1),
                state: AtomicIsize::new(0),
                flags: AtomicU8::new(0),
                writer: (AtomicU64::new(0), AtomicPtr::new(std::ptr::null_mut())),
                component: None,
                layout: (layout, offset),
                type_name: std::any::type_name::<T>(),
            })
        };

        Self {
            data: unsafe { NonNull::new_unchecked(inner_ptr) },
        }
    }

    fn get_ref(&self) -> &ComponentInner {
        // Safety: data is guaranteed to be valid as long as we exist.
        unsafe { self.data.as_ref() }
    }

    unsafe fn get_mut_ref(&mut self) -> &mut ComponentInner {
        // Safety: data is guaranteed to be valid as long as we exist. caller must ensure unique access.
        unsafe { self.data.as_mut() }
    }

    /// Checks if the component has been orphaned (i.e., removed from its parent store).
    pub fn is_orphaned(&self) -> bool {
        let inner = unsafe { self.data.as_ref() };
        let flags = LockState::from_bits_truncate(inner.flags.load(Ordering::Relaxed));
        flags.contains(LockState::ORPHANED)
    }

    /// Marks the component as orphaned.
    pub(crate) fn orphan(&self) {
        let inner = unsafe { self.data.as_ref() };
        inner
            .flags
            .fetch_or(LockState::ORPHANED.bits(), Ordering::Relaxed);
    }

    /// Drops the component. The caller must ensure that there are no outstanding references.
    pub(crate) unsafe fn drop_component(&mut self) {
        let inner = unsafe { self.get_mut_ref() };
        if let Some(component_ptr) = inner.component {
            // Drop the component
            let component_ref = component_ptr.as_ptr();
            inner.component = None;
            unsafe { std::ptr::drop_in_place(component_ref) };
        }
    }

    unsafe fn inner_ref(&self) -> &(dyn Any + Send + Sync) {
        // SAFETY: The ComponentPtr ensures that the inner is alive as long as there are strong references.
        unsafe {
            self.data
                .as_ref()
                .component
                .as_ref()
                .expect("ComponentPtr::is: Component not present")
                .as_ref()
        }
    }

    unsafe fn inner_mut(&mut self) -> &mut (dyn Any + Send + Sync) {
        // SAFETY: The caller must ensure unique access to the component.
        let inner = unsafe { self.get_mut_ref() };
        unsafe {
            inner
                .component
                .as_mut()
                .expect("ComponentPtr::inner_mut: Component not present")
                .as_mut()
        }
    }

    /// Downgrades the strong pointer to a weak pointer.
    pub fn downgrade(self) -> WeakComponentPtr {
        let inner = unsafe { self.data.as_ref() };
        inner.weak.fetch_add(1, Ordering::Relaxed);
        // SAFETY: ^
        unsafe { WeakComponentPtr::new(self.data) }
    }

    /// Attempts to get a read guard for the component of type T.
    #[track_caller]
    pub fn try_read<T: 'static>(&self) -> Result<Option<ComponentReadGuard<T>>, TypeMismatchError> {
        let inner = unsafe { self.data.as_ref() };
        if inner.component.is_none() {
            return Ok(None);
        }

        if unsafe { inner.component.unwrap().as_ref() }.is::<T>() {
            // SAFETY: We just checked that the type matches.
            unsafe { Ok(Some(ComponentReadGuard::lock(self.clone()))) }
        } else {
            Err(TypeMismatchError::new(
                std::any::type_name::<T>(),
                inner.type_name,
            ))
        }
    }

    /// Gets a read guard for the component of type T, panicking on type mismatch.
    #[track_caller]
    pub fn read<T: 'static>(&self) -> ComponentReadGuard<T> {
        self.try_read::<T>()
            .expect("ComponentPtr::read: Type mismatch when getting component")
            .expect("ComponentPtr::read: Component not initialized")
    }

    /// Attempts to get a write guard for the component of type T.
    #[track_caller]
    pub fn try_write<T: 'static>(
        &self,
    ) -> Result<Option<ComponentWriteGuard<T>>, TypeMismatchError> {
        let inner = unsafe { self.data.as_ref() };
        if inner.component.is_none() {
            return Ok(None);
        }

        if unsafe { inner.component.unwrap().as_ref() }.is::<T>() {
            // SAFETY: We just checked that the type matches.
            unsafe {
                Ok(Some(ComponentWriteGuard::lock(
                    self.clone(),
                    Location::caller(),
                )))
            }
        } else {
            Err(TypeMismatchError::new(
                std::any::type_name::<T>(),
                inner.type_name,
            ))
        }
    }

    /// Gets a write guard for the component of type T, panicking on type mismatch.
    #[track_caller]
    pub fn write<T: 'static>(&self) -> write::ComponentWriteGuard<T> {
        self.try_write::<T>()
            .expect("ComponentPtr::write: Type mismatch when getting component")
            .expect("ComponentPtr::write: Component not initialized")
    }

    /// Checks if the component is of type T.
    pub fn is<T: 'static>(&self) -> bool {
        let inner = unsafe { self.data.as_ref() };
        unsafe {
            inner
                .component
                .expect("ComponentPtr::is: Component not present")
                .as_ref()
        }
        .is::<T>()
    }

    /// Initializes the component with the given value.
    pub fn initialize<T: Send + Sync + 'static>(&mut self, component: T) -> Option<()> {
        let inner = unsafe { self.data.as_mut() };

        inner
            .flags
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |flags| {
                let mut state = LockState::from_bits_truncate(flags);
                if state.contains(LockState::IS_INIT) {
                    None
                } else {
                    state.insert(LockState::IS_INIT);
                    Some(state.bits())
                }
            })
            .ok()?;

        let component_ptr =
            unsafe { self.data.cast::<u8>().add(inner.layout.1).as_ptr() as *mut T };
        unsafe {
            component_ptr.write(component);
        }

        let component_trait_ptr: *mut (dyn Any + Send + Sync) = component_ptr;
        inner.component = Some(unsafe { NonNull::new_unchecked(component_trait_ptr) });
        Some(())
    }

    // Manually decrement the strong/weak counts, dropping the component if strong reaches zero.
    pub unsafe fn release(&self) {
        let inner = unsafe { self.data.as_ref() };
        if inner.strong.fetch_sub(1, Ordering::Release) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            unsafe {
                let mut self_mut = self.clone();
                self_mut.drop_component();
            }
            if inner.weak.fetch_sub(1, Ordering::Release) == 1 {
                std::sync::atomic::fence(Ordering::Acquire);
                unsafe {
                    std::alloc::dealloc(self.data.as_ptr() as *mut u8, inner.layout.0);
                }
            }
        }
    }

    /// Manually increment the strong/weak counts to retain the component.
    pub unsafe fn retain(&self) {
        let inner = unsafe { self.data.as_ref() };
        // keep in mind we use both strong and weak counts to keep the inner alive
        // weak is defined as keeping the enclosing structure alive, strong keeps the component alive
        inner.strong.fetch_add(1, Ordering::Relaxed);
        inner.weak.fetch_add(1, Ordering::Relaxed);
    }
}

impl Clone for ComponentPtr {
    fn clone(&self) -> Self {
        let inner = unsafe { self.data.as_ref() };
        inner.strong.fetch_add(1, Ordering::Relaxed);
        Self { data: self.data }
    }
}

impl Drop for ComponentPtr {
    fn drop(&mut self) {
        let inner = unsafe { self.data.as_ref() };
        if inner.strong.fetch_sub(1, Ordering::Release) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            unsafe {
                self.drop_component();
            }
            if inner.weak.fetch_sub(1, Ordering::Release) == 1 {
                std::sync::atomic::fence(Ordering::Acquire);
                unsafe {
                    std::alloc::dealloc(self.data.as_ptr() as *mut u8, inner.layout.0);
                }
            }
        }
    }
}

impl fmt::Debug for ComponentPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = unsafe { self.data.as_ref() };
        f.debug_struct("ComponentPtr")
            .field("type", &inner.type_name)
            .field("strong", &inner.strong.load(Ordering::Relaxed))
            .field("weak", &inner.weak.load(Ordering::Relaxed))
            .finish()
    }
}

fn create_component_inner_layout<T: Send + Sync + 'static>() -> (Layout, usize) {
    let inner_layout = Layout::new::<ComponentInner>();
    let data_layout = Layout::new::<T>();
    let (layout, offset) = inner_layout.extend(data_layout).unwrap();
    let layout = layout.pad_to_align();
    (layout, offset)
}
struct ComponentInner {
    // strong reference count
    strong: AtomicUsize,
    // weak reference count. prevents drop of everything but `component`
    weak: AtomicUsize,
    // reader-writer lock
    // readers
    // panics under the following conditions:
    // readers > 0 && strong == 0 ; this is more of a sanity check, and might end up as a `debug_assert!`
    // readers == AtomicIsize::MAX
    // if readers == -1, no readers can be acquired (meaning a writer is being acquired)
    // -1: possible writer active, no read locks can be acquired
    // 0: no readers, a writer can be acquired
    // >0: number of active readers
    state: AtomicIsize,
    // (tid, location) of the writer. location is only safe to read if tid == current_tid
    writer: (AtomicU64, AtomicPtr<Location<'static>>),
    flags: AtomicU8, // LockState
    // the actual component
    // this might seem strange, but whenever ComponentInner is allocated, the component is allocated inline after it.
    // we use a pointer here because after strong == 0, we want to be able to drop the component but keep the rest of the structure alive for weak refs.
    //
    // if you look at the std implementation of Arc, you'll see that they just append a T after the ArcInner struct in memory as well.
    // but i do actually take issue to this, because it means that you can safely access uninitialized memory, which given the scope isn't a huge deal,
    // but it still feels a bit icky.
    //
    // you might also ask like i did "why not just use MaybeUninit here?" and the answer is that
    // MaybeUninit<T: Sized>, so putting anything `dyn` in there would be a no-go.
    component: Option<NonNull<dyn Any + Send + Sync>>,
    // layout of the entire allocation, used for deallocation
    // (layout, offset to component)
    layout: (Layout, usize),
    // for debugging purposes, store the type name of the component
    type_name: &'static str,
}

unsafe impl Send for ComponentPtr {}
unsafe impl Sync for ComponentPtr {}

#[track_caller]
fn check_deadlock(state: &ComponentInner, lock_type: &str) {
    let tid = state.writer.0.load(Ordering::Relaxed);
    let this = thread::current().id().as_u64().get();

    // while `tid` could be modified between the load and the comparison,
    // the only time this check matters is if they are equal.
    // if they are equal, we know for sure we are deadlocked.
    if tid == this {
        let location = state.writer.1.load(Ordering::Relaxed);
        let location = unsafe { location.as_ref() };
        panic!(
            "Deadlock detected: thread attempted to acquire {} lock while holding write lock: {:?}",
            lock_type, location
        );
    }
}

bitflags! {
     struct LockState: u8 {
        /// The component has been orphaned (removed from its parent store).
        const ORPHANED = 1 << 0;
        /// a handle for a non existent component is waiting for initialization
        const IS_INIT = 1 << 1;
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Type mismatch: expected {expected}, found {found}")]
pub struct TypeMismatchError {
    expected: &'static str,
    found: &'static str,
}

impl TypeMismatchError {
    pub fn new(expected: &'static str, found: &'static str) -> Self {
        Self { expected, found }
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::component;

    use super::*;

    // INFO: Uses of `rc` here just mean the reference counted part of ComponentPtr

    #[test]
    fn test_rc_clone_single() {
        let component = ComponentPtr::new(42u32);
        let component_clone = component.clone();
        let inner = component.get_ref();
        assert_eq!(inner.strong.load(Ordering::Relaxed), 2);
        drop(component_clone);
        let inner = component.get_ref();
        assert_eq!(inner.strong.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_rc_heavy_multithread() {
        use std::sync::Arc;
        use std::thread;

        let component = Arc::new(ComponentPtr::new(42u32));
        let mut handles = vec![];

        for _ in 0..10 {
            let component_clone = Arc::clone(&component);
            let handle = thread::spawn(move || {
                for _ in 0..1000 {
                    let c = component_clone.clone();
                    drop(c);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let inner = component.get_ref();
        assert_eq!(inner.strong.load(Ordering::Relaxed), 1);
    }

    // this test covers regressions on both upgrade and drop of weak pointers, as well as strong count reaching zero
    #[test]
    fn test_rc_weak_drop() {
        let component = ComponentPtr::new(42u32);
        let weak = component.clone().downgrade();
        assert!(weak.upgrade().is_some());
        drop(component);
        assert!(weak.upgrade().is_none());
        let inner = unsafe { weak.data.as_ref() };
        assert_eq!(inner.weak.load(Ordering::Relaxed), 1);
        assert_eq!(inner.strong.load(Ordering::Relaxed), 0);
        assert!(inner.component.is_none());
    }

    #[test]
    /// test orphaning behavior
    fn test_rc_orphaning() {
        let component = ComponentPtr::new(42u32);
        assert!(!component.is_orphaned());
        component.orphan();
        assert!(component.is_orphaned());
    }

    #[test]
    fn test_deadlock_check_nodeadlock() {
        let component = ComponentPtr::new(42u32);
        let inner = component.get_ref();
        inner.writer.0.store(12345, Ordering::Relaxed);
        // should not panic
        check_deadlock(inner, "test");
    }

    #[test]
    #[should_panic(
        expected = "Deadlock detected: thread attempted to acquire abcd lock while holding write lock"
    )]
    fn test_deadlock_check_deadlock() {
        let component = ComponentPtr::new(42u32);
        let inner = component.get_ref();
        let this = thread::current().id().as_u64().get();
        inner.writer.0.store(this, Ordering::Relaxed);
        check_deadlock(inner, "abcd");
    }

    fn mixed_rw_heavy_multithread_read(component: ComponentPtr) {
        for i in 0..1000 {
            let guard = component.read::<u32>();
            assert!(*guard < 1000);
            if i % 10 == 0 {
                // yield occasionally to allow writers to proceed
                thread::yield_now();
            }
        }
    }

    fn mixed_rw_heavy_multithread_write(component: ComponentPtr) {
        let mut tr = rand::rng();
        for i in 0..1000 {
            let mut guard = component.write::<u32>();
            *guard = tr.random_range(0..1000);
            if i % 10 == 0 {
                // yield occasionally to allow writers to proceed
                thread::yield_now();
            }
        }
    }

    #[test]
    fn test_mixed_rw_heavy_multithread() {
        let ptr = ComponentPtr::new(100u32);

        let mut handles = vec![];

        for i in 0..10 {
            let ptr_clone = ptr.clone();
            if i % 2 == 0 {
                let handle = std::thread::spawn(move || {
                    mixed_rw_heavy_multithread_read(ptr_clone);
                });
                handles.push(handle);
            } else {
                let handle = std::thread::spawn(move || {
                    mixed_rw_heavy_multithread_write(ptr_clone);
                });
                handles.push(handle);
            }
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

    #[test]
    fn test_uninit_construction() {
        let ptr = ComponentPtr::uninitialized::<u32>();

        let state = ptr.get_ref();
        assert_eq!(state.strong.load(Ordering::Relaxed), 1);
        assert_eq!(state.weak.load(Ordering::Relaxed), 1);
        assert!(state.component.is_none());
        assert!(!ptr.is_orphaned());
        assert!(state.flags.load(Ordering::Relaxed) & LockState::IS_INIT.bits() == 0);
    }

    #[test]
    #[should_panic(expected = "ComponentPtr::read: Component not initialized")]
    fn test_uninit_read_guard_panic() {
        let ptr = ComponentPtr::uninitialized::<u32>();
        let _guard = ptr.read::<u32>();
    }

    #[test]
    #[should_panic(expected = "ComponentPtr::write: Component not initialized")]
    fn test_uninit_write_guard_panic() {
        let ptr = ComponentPtr::uninitialized::<u32>();
        let _guard = ptr.write::<u32>();
    }

    #[test]
    fn test_uninit_init_read() {
        let mut ptr = ComponentPtr::uninitialized::<u32>();
        assert!(ptr.initialize(55u32).is_some());

        let guard = ptr.read::<u32>();
        assert_eq!(*guard, 55u32);
        drop(guard);

        let inner_ref = ptr.get_ref();
        assert_eq!(
            inner_ref.state.load(std::sync::atomic::Ordering::Relaxed),
            0,
        );
        assert_eq!(
            inner_ref.flags.load(Ordering::Relaxed) & LockState::IS_INIT.bits(),
            LockState::IS_INIT.bits()
        );
    }

    #[test]
    fn test_uninit_init_write() {
        let mut ptr = ComponentPtr::uninitialized::<u32>();
        assert!(ptr.initialize(77u32).is_some());

        let guard = ptr.write::<u32>();
        assert_eq!(*guard, 77u32);
        drop(guard);

        let inner_ref = ptr.get_ref();
        assert_eq!(
            inner_ref.state.load(std::sync::atomic::Ordering::Relaxed),
            0,
        );
        assert_eq!(
            inner_ref.flags.load(Ordering::Relaxed) & LockState::IS_INIT.bits(),
            LockState::IS_INIT.bits()
        );
    }
}
