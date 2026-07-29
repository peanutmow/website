use wasm_bindgen::prelude::*;
use web_sys::console;

mod fluid;
mod render;

use fluid::FluidSim;
use render::AsciiRenderer;

/// Main WASM entry point — initializes the ASCII water simulation
#[wasm_bindgen]
pub struct WaterSimApp {
    fluid: FluidSim,
    renderer: AsciiRenderer,
    cols: u32,
    rows: u32,
    mouse_x: f64,
    mouse_y: f64,
    prev_mouse_x: f64,
    prev_mouse_y: f64,
    mouse_down: bool,
}

#[wasm_bindgen]
impl WaterSimApp {
    pub fn new(cols: u32, rows: u32) -> Self {
        console::log_1(&"WaterSimApp initialized (Rust)".into());
        WaterSimApp {
            fluid: FluidSim::new(cols, rows),
            renderer: AsciiRenderer::new(cols, rows),
            cols,
            rows,
            mouse_x: -1.0,
            mouse_y: -1.0,
            prev_mouse_x: -1.0,
            prev_mouse_y: -1.0,
            mouse_down: false,
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.cols = cols;
        self.rows = rows;
        self.fluid.resize(cols, rows);
        self.renderer.resize(cols, rows);
    }

    pub fn cols(&self) -> u32 { self.cols }
    pub fn rows(&self) -> u32 { self.rows }

    pub fn set_mouse(&mut self, x: f64, y: f64, down: bool) {
        self.prev_mouse_x = self.mouse_x;
        self.prev_mouse_y = self.mouse_y;
        self.mouse_x = x;
        self.mouse_y = y;
        self.mouse_down = down;

        if down && x >= 0.0 && y >= 0.0 && self.prev_mouse_x >= 0.0 && self.prev_mouse_y >= 0.0 {
            let dx = (x - self.prev_mouse_x) * 3.0;
            let dy = (y - self.prev_mouse_y) * 3.0;
            self.fluid.add_force(x as u32, y as u32, dx as f32, dy as f32);
            self.fluid.add_density(x as u32, y as u32, 5.0);
        }
    }

    pub fn step(&mut self) {
        self.fluid.step();
    }

    /// Render the current fluid state to an ASCII string
    pub fn render(&self) -> String {
        self.renderer.render(&self.fluid.density)
    }
}
