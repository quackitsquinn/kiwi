use std::cell::RefCell;

/// Internal representation of a resource.
/// This contains various metadata about the resource.
#[derive(Debug)]
pub struct ResourceNode {
    pub type_name: &'static str,
    pub data: RefCell<Box<dyn std::any::Any>>,
}

impl ResourceNode {
    /// Creates a new ResourceNode from the given data.
    pub fn new<T: 'static + std::any::Any>(data: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            data: RefCell::new(Box::new(data)),
        }
    }

    /// Downcasts the resource to the specified type.
    ///
    /// # Safety
    /// The caller must ensure that the type T matches the actual type of the resource.
    pub unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> std::cell::Ref<'_, T> {
        std::cell::Ref::map(self.data.borrow(), |b| {
            // Currently we do only use downcast_ref, but in the future this might be turned into a manual pointer cast for performance reasons.
            // Don't make safety promises you can't keep!
            b.downcast_ref::<T>()
                .expect("Resource type mismatch during downcast")
        })
    }

    /// Downcasts the resource to the specified mutable type.
    ///
    /// # Safety
    /// The caller must ensure that the type T matches the actual type of the resource.
    pub unsafe fn downcast_mut_unchecked<T: 'static>(&self) -> std::cell::RefMut<'_, T> {
        std::cell::RefMut::map(self.data.borrow_mut(), |b| {
            // Currently we do only use downcast_mut, but in the future this might be turned into a manual pointer cast for performance reasons.
            // Don't make safety promises you can't keep!
            b.downcast_mut::<T>()
                .expect("Resource type mismatch during downcast")
        })
    }
}
