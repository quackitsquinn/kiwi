use std::sync::Arc;

use glam::Vec3;

pub use anyhow;
pub use bytemuck;
pub use glam;
pub use parking_lot;
pub use wgpu;
pub use winit;

// even though the project im using this for is using tokio, smol is more lightweight and fits better with the rest of kiwi's design
// (game with heavy use of networking.. very very excited to work on this!)
pub use smol;

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
