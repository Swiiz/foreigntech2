pub mod app;
pub mod constants;
pub mod game;
pub mod graphics;
pub mod utils;

asset_tree::asset_tree!(assets {});

pub static ASSETS: std::sync::LazyLock<assets::AssetsFolder> = std::sync::LazyLock::new(|| {
    <assets::AssetsFolder as asset_tree::Asset>::load(
        &<asset_tree::loader::StdOsLoader as asset_tree::loader::AssetLoader>::new(String::new())
            .expect("Assets platform is not supported"),
    )
    .expect("Failed to load game assets")
});
