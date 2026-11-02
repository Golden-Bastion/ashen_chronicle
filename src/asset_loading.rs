//! Runtime loading for protected base assets and mod-owned assets.
//! Bare paths are base-game references and can be replaced by the highest-priority
//! enabled mod. Explicit mod references use stable manifest IDs: mod:<id>/<path>.

use std::fs;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::{Assets, Handle, Image};

pub(crate) fn load_image_from_data(
    images: &mut Assets<Image>,
    reference: &str,
) -> Result<Handle<Image>, String> {
    let path = crate::mod_manager::resolve_asset_reference(reference)
        .map_err(|error| error.to_string())?;
    if path.extension().and_then(|value| value.to_str()) != Some("png") {
        return Err(format!("only PNG image assets are supported: {}", path.display()));
    }
    let bytes = fs::read(&path)
        .map_err(|error| format!("could not read asset {}: {error}", path.display()))?;
    let mut image = Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::default(),
    )
    .map_err(|error| format!("could not decode PNG asset {}: {error}", path.display()))?;
    image.sampler = ImageSampler::nearest();
    Ok(images.add(image))
}
