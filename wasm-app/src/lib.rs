#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod blog;
#[cfg(target_arch = "wasm32")]
mod gallery;
#[cfg(target_arch = "wasm32")]
mod landing;
#[cfg(target_arch = "wasm32")]
mod pages;
#[cfg(target_arch = "wasm32")]
mod site_window;
#[cfg(target_arch = "wasm32")]
mod water;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start() {
    #[cfg(target_arch = "wasm32")]
    app::start();
}
