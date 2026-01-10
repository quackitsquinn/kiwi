use std::{
    cell::RefCell,
    fmt::Debug,
    rc::{Rc, Weak},
    sync::Arc,
};

use log::info;

use crate::ReadOnlyString;

/// A general purpose callback proxy for managing and invoking callbacks.
#[derive(Clone)]
pub struct Proxy<T: Copy>(Arc<CallbackProxy<T>>);

type RefVec<T> = RefCell<Vec<T>>;

pub type TargetHandle<T> = Rc<RefCell<dyn FnMut(T)>>;

struct CallbackProxy<Args>
where
    Args: Copy,
{
    targets: RefVec<CallbackTarget<Args>>,
    suspended: RefCell<bool>,
}

// TODO: Reduce Copy into Clone?
impl<Args> Proxy<Args>
where
    Args: Copy,
{
    /// Creates a new GlfwCallbackProxy.
    pub fn new() -> Self {
        Self(Arc::new(CallbackProxy {
            targets: RefCell::new(Vec::new()),
            suspended: RefCell::new(false),
        }))
    }

    /// Adds a new target callback to be invoked.
    pub fn add_target(
        &self,
        callback: impl FnMut(Args) + 'static,
        label: Option<ReadOnlyString>,
    ) -> TargetHandle<Args> {
        let rc_callback: Rc<RefCell<dyn FnMut(Args)>> = Rc::new(RefCell::new(callback));
        let weak_callback = Rc::downgrade(&rc_callback);
        self.0
            .targets
            .borrow_mut()
            .push(CallbackTarget::new(weak_callback, label));
        rc_callback
    }

    /// Invokes all registered target callbacks with the given arguments.
    pub fn invoke(&self, args: Args) {
        if *self.0.suspended.borrow() {
            return;
        }
        let mut targets = self.0.targets.borrow_mut();
        targets.retain(|target| {
            if let Some(callback_rc) = target.callback.upgrade() {
                let mut callback = callback_rc.borrow_mut();
                callback(args);
                true
            } else {
                info!(
                    "Removing dead callback target: {}",
                    target.label.as_ref().map(|a| &a[..]).unwrap_or("unknown")
                );
                false
            }
        });
    }

    pub fn suspend(&self) {
        *self.0.suspended.borrow_mut() = true;
    }

    pub fn unsuspend(&self) {
        *self.0.suspended.borrow_mut() = false;
    }
}

impl<T> Debug for Proxy<T>
where
    T: 'static + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlfwCallbackProxy")
            .field("targets_count", &self.0.targets.borrow().len())
            .finish()
    }
}

impl<T> Default for Proxy<T>
where
    T: Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct CallbackTarget<T>
where
    T: Copy,
{
    label: Option<ReadOnlyString>,
    callback: Weak<RefCell<dyn FnMut(T)>>,
}

impl<T> CallbackTarget<T>
where
    T: Copy,
{
    pub fn new(callback: Weak<RefCell<dyn FnMut(T)>>, label: Option<ReadOnlyString>) -> Self {
        Self { label, callback }
    }
}
