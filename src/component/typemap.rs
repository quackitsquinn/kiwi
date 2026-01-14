use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

use rustc_hash::FxBuildHasher;

/// A type map for storing resources of various types.
#[derive(Debug, Default)]
pub struct TypeMap {
    map: HashMap<TypeId, TypeContainer, FxBuildHasher>,
}

impl TypeMap {
    /// Creates a new, empty TypeMap.
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Inserts a resource into the TypeMap.
    pub fn insert<T: 'static + Send + Sync>(&mut self, resource: T) {
        self.map
            .insert(std::any::TypeId::of::<T>(), TypeContainer::new(resource));
    }

    /// Retrieves a reference to a resource of the specified type.
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.map
            .get(&std::any::TypeId::of::<T>())
            .and_then(|node| node.data.downcast_ref::<T>())
    }

    /// Removes a resource of the specified type from the TypeMap.
    pub fn remove<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.map
            .remove(&std::any::TypeId::of::<T>())
            .and_then(|node| node.data.downcast::<T>().ok().map(|b| *b))
    }

    /// Retrieves a mutable reference to a resource of the specified type.
    pub fn get_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&std::any::TypeId::of::<T>())
            .and_then(|node| node.data.downcast_mut::<T>())
    }

    /// Clears all resources from the TypeMap.
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

/// An immutable type map for storing resources of various types. Unlike `TypeMap`, this is `Arc`-based and only allows immutable access BUT allows
/// taking "handles" to resources that can be cloned and used elsewhere.
#[derive(Debug, Default)]
pub struct ImmutableTypeMap {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>, FxBuildHasher>,
}

impl ImmutableTypeMap {
    /// Creates a new, empty ImmutableTypeMap.
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Inserts a resource into the ImmutableTypeMap.
    /// The resource is wrapped in an `Arc`.
    pub fn insert<T: 'static + Send + Sync>(&mut self, resource: T) {
        self.map
            .insert(std::any::TypeId::of::<T>(), Arc::new(resource));
    }

    /// Retrieves an `Arc` to a resource of the specified type.
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.map
            .get(&std::any::TypeId::of::<T>())
            .and_then(|node| node.clone().downcast::<T>().ok())
    }

    /// Removes a resource of the specified type from the ImmutableTypeMap.
    /// The resource is returned as an `Arc`.
    pub fn remove<T: 'static + Send + Sync>(&mut self) -> Option<Arc<T>> {
        self.map
            .remove(&std::any::TypeId::of::<T>())
            .and_then(|node| node.downcast::<T>().ok())
    }

    /// Returns an iterator over the TypeIds of the stored resources.
    pub fn keys(&self) -> impl Iterator<Item = &TypeId> {
        self.map.keys()
    }
}

struct TypeContainer {
    type_name: &'static str,
    data: Box<dyn Any + Send + Sync>,
}

impl TypeContainer {
    fn new<T: 'static + Send + Sync>(data: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            data: Box::new(data),
        }
    }
}

impl Debug for TypeContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}>", self.type_name)
    }
}
