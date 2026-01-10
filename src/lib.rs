use std::sync::Arc;

use glam::Vec3;

/// A read-only string type.
pub type ReadOnlyString = Arc<str>;
/// A read-only slice type.
pub type ReadOnly<T> = Arc<[T]>;
/// A position in the world, in floating-point coordinates.
pub type FloatPosition = Vec3;

pub mod assets;
pub mod component;
pub mod graphics;
pub mod input;
pub mod shared;
