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

pub use typemap::{ImmutableTypeMap, TypeMap};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard};
//use resource::ResourceNode;
use rustc_hash::FxBuildHasher;

use crate::component::resource::ComponentPtr;

type ResourceMap = HashMap<TypeId, ComponentPtr, FxBuildHasher>;

pub type ComponentReadGuard<'a, T> = MappedRwLockReadGuard<'a, T>;
pub type ComponentWriteGuard<'a, T> = MappedRwLockWriteGuard<'a, T>;

/// A database for storing components of various types.
#[derive(Default)]
pub struct ComponentStore {
    map: Arc<ResourceMap>,
    public_ref: Arc<OnceLock<ComponentStore>>,
}

impl ComponentStore {
    /// Creates a new, empty component database.
    pub fn new() -> Self {
        Self {
            map: Arc::new(HashMap::default()),
            public_ref: Arc::new(OnceLock::new()),
        }
    }

    /// Finalizes the initialization of the component database.
    pub fn finish_initialization(&self) {
        let _ = self.public_ref.set(Self {
            map: self.map.clone(),
            public_ref: self.public_ref.clone(),
        });
    }

    /// Function for internal use to get the resource map. (must not be exposed publicly)
    fn get_map(&self) -> &Arc<ResourceMap> {
        &self.map
    }

    /// Inserts a component into the database.
    ///
    /// There must be no other references to the database when calling this method.
    pub fn insert<T: 'static + Send + Sync>(&mut self, component: T) -> ComponentHandle<T> {
        if self.map.contains_key(&TypeId::of::<T>()) {
            panic!(
                "Component of type {} already exists in State",
                std::any::type_name::<T>()
            );
        }

        let mut_map =
            Arc::get_mut(&mut self.map).expect("Cannot insert component into shared State");

        mut_map.insert(TypeId::of::<T>(), ComponentPtr::new(component));
        self.handle_for::<T>()
    }

    /// Creates a handle for a component of the specified type.
    pub fn handle_for<T: 'static>(&self) -> ComponentHandle<T> {
        ComponentHandle::new(self.handle())
    }

    /// Creates a handle to the component map.
    pub fn handle(&self) -> ComponentStoreHandle {
        ComponentStoreHandle::new(self)
    }
}

impl Debug for ComponentStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct TyDbg(&'static str);
        impl Debug for TyDbg {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        let mut type_names: Vec<TyDbg> = vec![];
        for component in self.map.iter() {}
        f.debug_struct("State")
            .field("resources", &type_names)
            .finish()
    }
}

/// A handle to a component stored in a `ComponentDB`.
pub struct ComponentHandle<T: 'static> {
    handle: ComponentStoreHandle,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ComponentHandle<T> {
    fn new(state_handle: ComponentStoreHandle) -> Self {
        Self {
            handle: state_handle,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Gets a reference to the component.
    pub fn get(&self) -> ComponentReadGuard<'_, T> {
        self.handle.get::<T>()
    }

    /// Gets a mutable reference to the component.
    pub fn get_mut(&self) -> ComponentWriteGuard<'_, T> {
        self.handle.get_mut::<T>()
    }
}

impl<T> Debug for ComponentHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResourceHandle<{}>", std::any::type_name::<T>())
    }
}

impl<T> Clone for ComponentHandle<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

/// A handle to a ComponentMap that allows checking its state.
#[derive(Clone)]
pub struct ComponentStoreHandle {
    // TODO: Figure out a way to optimize this into a single pointer sized field.
    // This is gonna need some unsafe code and weird pointer tagging so this is a later task.
    handle: OnceLock<Arc<ResourceMap>>,
    global_handle: Arc<OnceLock<ComponentStore>>,
}

impl ComponentStoreHandle {
    pub fn new(component_map: &ComponentStore) -> Self {
        Self {
            handle: OnceLock::new(),
            global_handle: component_map.public_ref.clone(),
        }
    }

    fn get_map(&self) -> &Arc<ResourceMap> {
        self.handle.get_or_init(|| {
            let global = self
                .global_handle
                .get()
                .expect("StateHandle used before State was fully initialized");
            global.map.clone()
        })
    }

    /// Creates a handle for a component of the specified type.
    pub fn handle_for<T: 'static>(&self) -> ComponentHandle<T> {
        ComponentHandle::new(self.clone())
    }
}

impl Debug for ComponentStoreHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateHandle").finish()
    }
}

mod get_impls {
    use crate::component::{
        ComponentReadGuard, ComponentStore, ComponentStoreHandle, ComponentWriteGuard,
    };

    macro_rules! impl_get {
        () => {
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
        };
    }

    impl ComponentStoreHandle {
        impl_get!();
    }

    impl ComponentStore {
        impl_get!();
    }
}
