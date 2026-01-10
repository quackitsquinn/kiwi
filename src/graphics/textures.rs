use std::collections::HashMap;

use wgpu::{TextureFormat, TextureUsages, naga::back::msl::sampler};

use crate::{
    ReadOnly, ReadOnlyString,
    component::{ComponentHandle, ComponentStore, ComponentStoreHandle},
    graphics::{
        image::Image,
        lowlevel::{WgpuRenderer, texture::Texture},
    },
};

/// A structure managing a collection of textures.
/// This is effectively a texture atlas, packing multiple textures into a single GPU texture. The difference
/// is that this uses texture arrays instead of a single large texture, which greatly simplifies everything. The only limitation
/// is that all textures must have the same dimensions.
pub struct TextureCollection {
    textures: HashMap<String, TextureHandle>,
    buf: Vec<ReadOnly<u8>>,
    gpu_texture: Option<Texture>,
    label: Option<ReadOnlyString>,
    dimensions: (u32, u32),
    sampler: wgpu::Sampler,
    handle: ComponentHandle<WgpuRenderer>,
}

impl TextureCollection {
    pub fn new(
        state: &ComponentStore,
        label: Option<impl Into<ReadOnlyString>>,
        dimensions: (u32, u32),
        sampler: wgpu::Sampler,
    ) -> Self {
        Self {
            textures: HashMap::new(),
            buf: Vec::new(),
            gpu_texture: None,
            label: label.map(|l| l.into()),
            handle: state.handle_for::<WgpuRenderer>(),
            dimensions,
            sampler,
        }
    }

    /// Adds a new texture from raw RGBA8 data.
    pub fn add_texture(&mut self, name: &str, data: &Image) -> TextureHandle {
        let handle = TextureHandle::single(self.buf.len() as u32);
        self.buf.push(data.pixel_bytes().clone());
        self.textures.insert(name.to_string(), handle);
        handle
    }

    /// Adds multiple textures from an iterator of (name, data) pairs.
    pub fn add_textures<'a>(
        &mut self,
        name: &str,
        textures: impl IntoIterator<Item = &'a Image>,
    ) -> TextureHandle {
        let base = self.buf.len() as u32;
        let mut count = 0;
        for texture in textures {
            self.buf.push(texture.pixel_bytes().clone());
            count += 1;
        }
        let handle = TextureHandle::new(base, count);
        self.textures.insert(name.to_string(), handle);
        handle
    }

    pub fn push_invalid_texture(&mut self) -> TextureHandle {
        let mut data = vec![0u8; (self.dimensions.0 * self.dimensions.1 * 4) as usize];
        for (i, px) in data.chunks_mut(self.dimensions.0 as usize * 4).enumerate() {
            let is_bottom_half = i >= (self.dimensions.1 as usize / 2);
            for j in 0..(self.dimensions.0 as usize) {
                let offset = j * 4;
                let is_right_half = j >= (self.dimensions.0 as usize / 2);
                if !(is_bottom_half ^ is_right_half) {
                    px[offset + 3] = 255;
                    continue;
                }
                px[offset] = 255;
                px[offset + 1] = 0;
                px[offset + 2] = 255;
                px[offset + 3] = 255;
            }
        }

        self.buf.push(data.into());
        TextureHandle::single(self.buf.len() as u32 - 1)
    }

    /// Retrieves a texture handle by name.
    pub fn get_texture(&self, name: &str) -> Option<&TextureHandle> {
        self.textures.get(name)
    }

    /// Returns the GPU texture, creating it if it doesn't exist.
    pub fn gpu_texture(&mut self) -> Texture {
        if self.gpu_texture.is_some() {
            return self.gpu_texture.as_ref().unwrap().clone();
        }

        let texture = self.handle.get().texture(
            self.label.as_deref(),
            &self.sampler,
            TextureFormat::Rgba8Unorm,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            self.dimensions,
            &self.buf,
        );

        self.gpu_texture = Some(texture);
        self.gpu_texture.as_ref().unwrap().clone()
    }
}

/// A handle to a texture within a TextureCollection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureHandle {
    pub base_layer: u32,
    pub count: u32,
}

impl TextureHandle {
    /// Creates a new TextureHandle.
    pub fn new(base_layer: u32, count: u32) -> Self {
        Self { base_layer, count }
    }

    /// Creates a TextureHandle for a single layer.
    pub fn single(layer: u32) -> Self {
        Self {
            base_layer: layer,
            count: 1,
        }
    }

    /// Creates a null TextureHandle.
    ///
    /// This just points to layer 0 with a count of 1. Typically where the missing texture resides.
    pub fn null() -> Self {
        Self {
            base_layer: 0,
            count: 1,
        }
    }

    /// Gets the texture array layer for the given index.
    /// Panics if the index is out of bounds.
    pub fn layer(&self, index: u32) -> u32 {
        assert!(index < self.count);
        self.base_layer + index
    }
}
