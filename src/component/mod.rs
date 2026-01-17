use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, OnceLock},
    thread::ThreadId,
};

pub mod handles;
mod resource;
mod typemap;

pub use handles::ComponentHandle;

pub use typemap::{ImmutableTypeMap, TypeMap};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock};
//use resource::ResourceNode;
use rustc_hash::FxBuildHasher;

use crate::component::resource::ComponentPtr;

type ResourceMap = HashMap<TypeId, ComponentPtr, FxBuildHasher>;

pub use resource::{read::ComponentReadGuard, write::ComponentWriteGuard};

/// A database for storing components of various types.
#[derive(Default, Clone)]
pub struct ComponentStore {
    // previously, we just used an Arc that we didn't clone until finalized.
    // but this was really annoying to manage because handles didn't function until finalized.
    // so now we have both a modification map and a finalized map.
    // the ComponentStore can be cloned cheaply, and the handles can be created at any time.
    modification_map: Arc<RwLock<ResourceMap>>,
    map: Arc<OnceLock<ResourceMap>>,
}

pub type ComponentStoreHandle = ComponentStore;

impl ComponentStore {
    /// Creates a new, empty component database.
    pub fn new() -> Self {
        Self {
            modification_map: Default::default(),
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
    pub fn handle_for<T: 'static>(&self) -> ComponentHandle<T> {
        // if the read only map is initialized, use it
        if let Some(map) = self.map.get()
            && let Some(ptr) = map.get(&TypeId::of::<T>())
        {
            return ComponentHandle::new(ptr.clone());
        }

        // otherwise, use the modification map
        let guard = self.modification_map.read();
        let ptr = guard
            .get(&TypeId::of::<T>())
            .expect("Component not found in ComponentDB");

        ComponentHandle::new(ptr.clone())
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

impl ComponentStore {
    /// Gets a reference to a component of the specified type.
    pub fn get_checked<T: 'static>(&self) -> Option<ComponentReadGuard<'_, T>> {
        todo!("later")
    }

    /// Gets a reference to a component of the specified type.
    pub fn get<T: 'static>(&self) -> ComponentReadGuard<'_, T> {
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
    pub fn get_mut_checked<T: 'static>(&self) -> Option<ComponentWriteGuard<'_, T>> {
        todo!("later")
    }

    /// Gets a mutable reference to a component of the specified type.
    pub fn get_mut<T: 'static>(&self) -> ComponentWriteGuard<'_, T> {
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
