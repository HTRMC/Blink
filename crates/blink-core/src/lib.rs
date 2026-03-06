use wasm_bindgen::prelude::*;

mod buffer;
mod editor;
mod font_atlas;
mod icon_atlas;
mod renderer;
mod sidebar;
mod syntax;

pub use editor::Editor;
pub use sidebar::SidebarRenderer;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).expect("Failed to init logger");
    log::info!("Blink core initialized");
}
