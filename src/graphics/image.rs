use std::{fmt::Debug, sync::Arc};

use anyhow::anyhow;
use image::ImageBuffer;

use crate::ReadOnly;

#[derive(PartialEq, Eq)]
pub struct Image {
    image: image::ImageBuffer<image::Rgba<u8>, ReadOnly<u8>>,
    pixel_bytes: ReadOnly<u8>,
}

impl Image {
    /// Creates an Image from raw bytes.
    pub fn from_mem(raw_bytes: &[u8]) -> anyhow::Result<Self> {
        let image = image::load_from_memory(raw_bytes)?;
        let rgba_image = image.to_rgba8();
        let pixel_bytes: Arc<[u8]> = Arc::from(rgba_image.as_raw().as_slice());
        let image =
            ImageBuffer::from_raw(rgba_image.width(), rgba_image.height(), pixel_bytes.clone())
                .ok_or(anyhow!("failed to create image buffer!"))?;

        Ok(Self { image, pixel_bytes })
    }

    /// Creates an Image from a file path.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let image = image::open(path)?.to_rgba8();
        let pixel_bytes: Arc<[u8]> = Arc::from(image.as_raw().as_slice());
        let image = ImageBuffer::from_raw(image.width(), image.height(), pixel_bytes.clone())
            .ok_or(anyhow!("failed to create image buffer!"))?;

        Ok(Self { image, pixel_bytes })
    }

    /// Returns the dimensions of the image as (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        (self.image.width(), self.image.height())
    }

    /// Returns the raw pixel bytes of the image in RGBA format.
    pub fn pixel_bytes(&self) -> &ReadOnly<u8> {
        &self.pixel_bytes
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("dimensions", &self.dimensions())
            .finish()
    }
}

impl Clone for Image {
    fn clone(&self) -> Self {
        Self {
            image: self.image.clone(),
            pixel_bytes: self.pixel_bytes.clone(),
        }
    }
}
