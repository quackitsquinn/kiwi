#![feature(thread_id_value)]
use std::sync::Arc;

use glam::Vec3;

pub use anyhow;
pub use bytemuck;
pub use glam;
pub use parking_lot;
pub use rustc_hash;
pub use wgpu;
pub use winit; // fast hash map implementation

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
pub mod prelude {
    pub use crate::FloatPosition;
    pub use crate::ReadOnly;
    pub use crate::ReadOnlyString;

    pub use crate::anyhow;
    pub use crate::bytemuck;
    pub use crate::glam::{self, Mat4, Quat, Vec2, Vec3, Vec4};
    pub use crate::parking_lot;
    pub use crate::smol;
    pub use crate::wgpu;
    pub use crate::winit;

    pub use crate::assets::*;
    pub use crate::component::*;
    pub use crate::graphics::{
        CardinalDirection,
        camera::Camera as RawCamera,
        lowlevel::{
            WgpuRenderer,
            buf::{IndexBuffer, IndexLayout, UniformBuffer, VertexBuffer, VertexLayout},
            pipeline::{PipelineBuilder, WgpuPipeline},
            shader::ShaderProgram,
        },
    };
    pub use crate::input::*;
    pub use crate::shared::*;
}
