/// ASCII renderer for the fluid simulation density field
pub struct AsciiRenderer {
    cols: u32,
    rows: u32,
    chars: Vec<char>,
    buffer: String,
}

impl AsciiRenderer {
    pub fn new(cols: u32, rows: u32) -> Self {
        let chars = " .,-~:=+*#%@".chars().collect();
        let buffer_size = (cols * rows + rows) as usize; // rows for newlines
        let buffer = String::with_capacity(buffer_size);
        AsciiRenderer { cols, rows, chars, buffer }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.cols = cols;
        self.rows = rows;
        let buffer_size = (cols * rows + rows) as usize;
        self.buffer = String::with_capacity(buffer_size);
    }

    /// Render density field to ASCII string
    pub fn render(&self, density: &[f32]) -> String {
        let mut output = String::with_capacity(self.buffer.capacity());
        let max_chars = self.chars.len() - 1;

        for j in 0..self.rows {
            for i in 0..self.cols {
                let idx = (i + j * self.cols) as usize;
                let d = if idx < density.len() { density[idx] } else { 0.0 };

                // Map density to character index
                let normalized = (d * max_chars as f32).min(max_chars as f32);
                let char_idx = normalized as usize;
                output.push(self.chars[char_idx]);
            }
            if j < self.rows - 1 {
                output.push('\n');
            }
        }

        output
    }
}
