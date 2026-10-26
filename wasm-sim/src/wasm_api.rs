use wasm_bindgen::prelude::*;

use crate::sim::Sim;

#[wasm_bindgen]
pub struct WaterSim {
    inner: Sim,
}

#[wasm_bindgen]
impl WaterSim {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, seed: u32) -> WaterSim {
        WaterSim {
            inner: Sim::new(cols, rows, seed),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &mut self,
        greeting: &str,
        buttons: &str,
        mode: &str,
        text: bool,
        typewriter: bool,
        page_hooks: bool,
        easter_eggs: bool,
        reduced_motion: bool,
    ) {
        let buttons: Vec<String> = if buttons.is_empty() {
            Vec::new()
        } else {
            buttons.split('\n').map(str::to_string).collect()
        };
        self.inner.configure(
            greeting,
            &buttons,
            mode,
            text,
            typewriter,
            page_hooks,
            easter_eggs,
            reduced_motion,
        );
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.inner.resize(cols, rows);
    }

    pub fn step(&mut self) {
        self.inner.step();
    }

    pub fn render(&mut self, hide_text: bool) -> String {
        self.inner.render(hide_text)
    }

    pub fn mode(&self) -> String {
        self.inner.mode().to_string()
    }

    pub fn set_mode(&mut self, mode: &str, text: Option<bool>) {
        self.inner.set_mode(mode, text);
    }

    pub fn set_text_visible(&mut self, visible: bool) {
        self.inner.set_text_visible(visible);
    }

    pub fn ramp(&self) -> String {
        self.inner.ramp().to_string()
    }

    pub fn layout_json(&self) -> String {
        self.inner.layout_json()
    }

    pub fn pointer_move(&mut self, gx: f64, gy: f64) {
        self.inner.pointer_move(gx, gy);
    }

    pub fn pointer_down(&mut self, gx: Option<f64>, gy: Option<f64>) {
        self.inner.pointer_down(gx, gy);
    }

    pub fn pointer_leave(&mut self) {
        self.inner.pointer_leave();
    }

    pub fn pointer_active(&mut self, active: bool) {
        self.inner.pointer_active(active);
    }

    pub fn inject_at(&mut self, gi: f64, gj: f64, intensity: f64) {
        self.inner.inject_at(gi, gj, intensity);
    }

    pub fn typewriter_start(&mut self) {
        self.inner.typewriter_start();
    }

    pub fn typewriter_tick(&mut self) {
        self.inner.typewriter_tick();
    }

    pub fn typewriter_done(&mut self) {
        self.inner.typewriter_done();
    }

    pub fn key(&mut self, key: &str) -> String {
        self.inner.key(key).to_string()
    }

    pub fn clear_egg(&mut self) {
        self.inner.clear_egg();
    }

    pub fn trigger_about_blank(&mut self) {
        self.inner.trigger_about_blank();
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn droplet(&mut self, ci: f64, cj: f64, amount: Option<f64>, radius: Option<f64>) {
        self.inner
            .droplet(ci, cj, amount.unwrap_or(0.9), radius.unwrap_or(4.0));
    }

    pub fn vortex(&mut self, ci: f64, cj: f64, strength: Option<f64>) {
        self.inner.vortex(ci, cj, strength.unwrap_or(0.4));
    }

    pub fn stir(&mut self) {
        self.inner.stir();
    }

    pub fn clear_masks(&mut self) {
        self.inner.clear_masks();
    }

    pub fn push_mask(&mut self, row: f64, col: f64, rows: f64, cols: f64, fade: f64) {
        self.inner.push_mask(row, col, rows, cols, fade);
    }

    pub fn apply_masks(&mut self) {
        self.inner.apply_masks();
    }

    pub fn sample_json(&self, i: i64, j: i64) -> String {
        self.inner.sample_json(i, j)
    }

    pub fn current_segments(&self) -> Vec<f32> {
        self.inner.current_segments()
    }

    pub fn cols(&self) -> u32 {
        self.inner.fluid.cols
    }

    pub fn rows(&self) -> u32 {
        self.inner.fluid.rows
    }

    pub fn dens_ptr(&self) -> u32 {
        self.inner.fluid.dens.as_ptr() as u32
    }

    pub fn u_ptr(&self) -> u32 {
        self.inner.fluid.u.as_ptr() as u32
    }

    pub fn v_ptr(&self) -> u32 {
        self.inner.fluid.v.as_ptr() as u32
    }
}
