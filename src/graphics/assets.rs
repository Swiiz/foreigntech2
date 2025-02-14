use std::string::FromUtf8Error;

use asset_tree::{asset_files, loader::AssetLoader};
use image::ImageError;

pub struct ModelFile(pub String);
pub struct MaterialFile(pub String);
pub struct TextureFile(pub image::DynamicImage);

asset_files!(ModelFile: "obj", MaterialFile: "mtl", TextureFile: "png",);

impl TryFrom<Vec<u8>> for ModelFile {
    type Error = FromUtf8Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(String::from_utf8(value)?))
    }
}

impl TryFrom<Vec<u8>> for MaterialFile {
    type Error = FromUtf8Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(String::from_utf8(value)?))
    }
}

impl TryFrom<Vec<u8>> for TextureFile {
    type Error = ImageError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(image::load_from_memory(&value)?))
    }
}
