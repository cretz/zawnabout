//! The client: input, camera, procedural visuals and rendering on top of the sim.
//! Platform hosts (native window, headless, browser) drive it.

pub mod camera;
pub mod feedback;
pub mod game;
pub mod gfx;
pub mod input;
pub mod map;
pub mod mesh;
pub mod models;
pub mod pose;
pub mod render;
pub mod script;
pub mod terrain_mesh;

pub use game::Game;
pub use gfx::Gfx;
pub use render::Renderer;
pub use script::{Output, Script};
pub use wgpu;
pub use zawnabout_sim as sim;

/// Picks the sRGB view of a surface format, so shaders always write linear color.
pub fn srgb_view_format(format: wgpu::TextureFormat) -> wgpu::TextureFormat {
    format.add_srgb_suffix()
}
