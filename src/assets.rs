use std::{collections::HashMap, sync::Arc};

use crate::{ReadOnlyString, graphics::image::Image};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssetStore {
    // TODO: Actual asset store implementation
    images: HashMap<ReadOnlyString, Image>,
}

impl AssetStore {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    /// Adds an image to the asset store.
    pub fn add_image(
        &mut self,
        name: impl Into<ReadOnlyString>,
        data: &[u8],
    ) -> anyhow::Result<Image> {
        let image = Image::from_mem(data)?;
        self.images.insert(name.into(), image.clone()); // This clone is cheap due to Image using Arc internally
        Ok(image)
    }

    /// Retrieves an image by name.
    pub fn get_image(&self, name: &str) -> Option<Image> {
        self.images.get(name).cloned()
    }
}
