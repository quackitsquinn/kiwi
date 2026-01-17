use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Debug,
    ops::Deref,
    sync::{Arc, OnceLock},
    thread::ThreadId,
};

pub mod handles;
mod resource;
mod typemap;

pub use handles::ComponentHandle;

pub use typemap::{ImmutableTypeMap, TypeMap};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard};
//use resource::ResourceNode;
use rustc_hash::FxBuildHasher;

use crate::component::resource::ComponentPtr;

type ResourceMap = HashMap<TypeId, ComponentPtr, FxBuildHasher>;

pub use resource::{read::ComponentReadGuard, write::ComponentWriteGuard};

/// A database for storing components of various types.
#[derive(Clone)]
pub struct ComponentStore {
    /// A modification map used during initialization.
    modification_map: ComponentHandle<ResourceMap>,
    map: Arc<OnceLock<ResourceMap>>,
}

pub type ComponentStoreHandle = ComponentStore;

impl ComponentStore {
    /// Creates a new, empty component database.
    pub fn new() -> Self {
        Self {
            modification_map: ComponentHandle::standalone(Default::default()),
            map: Default::default(),
        }
    }

    /// Finalizes the initialization of the component database.
    pub fn finish_initialization(&self) {
        let map = self.modification_map.read().clone();
        self.map
            .set(map)
            .expect("ComponentDB finish_initialization called multiple times");
    }

    /// Inserts a component into the database.
    ///
    pub fn insert<T: 'static + Send + Sync>(&mut self, component: T) -> ComponentHandle<T> {
        if self.map.get().is_some() {
            panic!("Cannot insert component into finalized ComponentDB");
        }

        let mut guard = self.modification_map.write();
        if guard.contains_key(&TypeId::of::<T>()) {
            panic!(
                "Component of type {} already exists in State",
                std::any::type_name::<T>()
            );
        }

        let ptr = ComponentPtr::new(component);
        guard.insert(TypeId::of::<T>(), ptr.clone());

        self.handle_for::<T>()
    }

    /// Creates a handle for a component of the specified type.
    ///
    /// NOTE: Handles for non-existent components can be created; attempting to use them without inserting the component first will panic.
    pub fn handle_for<T: 'static + Send + Sync>(&self) -> ComponentHandle<T> {
        // if the read only map is initialized, use it
        if let Some(map) = self.map.get()
            && let Some(ptr) = map.get(&TypeId::of::<T>())
        {
            return ComponentHandle::new(ptr.clone());
        }

        // otherwise, use the modification map
        let guard = self.modification_map.read();
        if let Some(ptr) = guard.get(&TypeId::of::<T>()) {
            return ComponentHandle::new(ptr.clone());
        }

        // Are we init yet?
        let is_init = self.map.get().is_some();
        if is_init {
            panic!(
                "Component of type {} does not exist in ComponentDB",
                std::any::type_name::<T>()
            );
        }

        let ptr = ComponentPtr::uninitialized::<T>();

        self.modification_map
            .write()
            .insert(TypeId::of::<T>(), ptr.clone());

        ComponentHandle::new(ptr)
    }

    /// Creates a handle to the component map.
    #[deprecated = "use clone() instead"]
    pub fn handle(&self) -> ComponentStoreHandle {
        self.clone()
    }
}

impl Debug for ComponentStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(map) = self.map.get() {
            f.debug_struct("ComponentDB")
                .field("finalized", &true)
                .field("components", &map.values().collect::<Vec<_>>())
                .finish()
        } else {
            let guard = self.modification_map.read();
            f.debug_struct("ComponentDB")
                .field("finalized", &false)
                .field("components", &guard.values().collect::<Vec<_>>())
                .finish()
        }
    }
}

impl Default for ComponentStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentStore {
    /// Gets a reference to a component of the specified type.
    pub fn get_checked<T: 'static>(&self) -> Option<ComponentReadGuard<T>> {
        // if the read only map is initialized, use it
        if let Some(map) = self.map.get()
            && let Some(ptr) = map.get(&TypeId::of::<T>())
        {
            return Some(ptr.read());
        }

        if let Some(guard) = self.modification_map.read().get(&TypeId::of::<T>()) {
            return Some(guard.read());
        }

        None
    }

    /// Gets a reference to a component of the specified type.
    pub fn get<T: 'static>(&self) -> ComponentReadGuard<T> {
        if let Some(component) = self.get_checked::<T>() {
            component
        } else {
            panic!(
                "Component {} not found in ComponentDB",
                std::any::type_name::<T>()
            );
        }
    }

    /// Gets a mutable reference to a component of the specified type.
    pub fn get_mut_checked<T: 'static>(&self) -> Option<ComponentWriteGuard<T>> {
        // if the read only map is initialized, use it
        if let Some(map) = self.map.get()
            && let Some(ptr) = map.get(&TypeId::of::<T>())
        {
            return Some(ptr.write());
        }

        if let Some(guard) = self.modification_map.read().get(&TypeId::of::<T>()) {
            return Some(guard.write());
        }

        None
    }

    /// Gets a mutable reference to a component of the specified type.
    pub fn get_mut<T: 'static>(&self) -> ComponentWriteGuard<T> {
        if let Some(component) = self.get_mut_checked::<T>() {
            component
        } else {
            panic!(
                "Component {} not found in ComponentDB",
                std::any::type_name::<T>()
            );
        }
    }
}
