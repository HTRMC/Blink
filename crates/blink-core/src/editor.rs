use wasm_bindgen::prelude::*;
use crate::buffer::TextBuffer;
use crate::renderer::Renderer;

/// Cursor position in the editor.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
    pub offset: usize,
}

/// The main editor state, exposed to JavaScript.
#[wasm_bindgen]
pub struct Editor {
    buffer: TextBuffer,
    cursor: Cursor,
    renderer: Option<Renderer>,
    scroll_y: f32,
    viewport_height: f32,
}

#[wasm_bindgen]
impl Editor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Editor {
            buffer: TextBuffer::new(""),
            cursor: Cursor {
                line: 0,
                col: 0,
                offset: 0,
            },
            renderer: None,
            scroll_y: 0.0,
            viewport_height: 0.0,
        }
    }

    /// Initialize the WebGPU renderer on the given canvas with font data.
    pub async fn init_renderer(
        &mut self,
        canvas_id: &str,
        font_data: &[u8],
    ) -> Result<(), JsValue> {
        let renderer = Renderer::new(canvas_id, font_data).await?;
        self.renderer = Some(renderer);
        Ok(())
    }

    /// Set the text content of the buffer.
    pub fn set_content(&mut self, text: &str) {
        self.buffer = TextBuffer::new(text);
        self.cursor = Cursor {
            line: 0,
            col: 0,
            offset: 0,
        };
    }

    /// Get the current text content.
    pub fn get_content(&self) -> String {
        self.buffer.content()
    }

    /// Insert text at the current cursor position.
    pub fn insert_text(&mut self, text: &str) {
        self.buffer.insert(self.cursor.offset, text);
        self.cursor.offset += text.len();
        self.recalculate_cursor_position();
    }

    /// Delete `count` characters before the cursor (backspace).
    pub fn delete_backward(&mut self, count: usize) {
        let actual = count.min(self.cursor.offset);
        if actual == 0 {
            return;
        }
        self.buffer.delete(self.cursor.offset - actual, actual);
        self.cursor.offset -= actual;
        self.recalculate_cursor_position();
    }

    /// Get the number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    /// Get the cursor line.
    pub fn cursor_line(&self) -> usize {
        self.cursor.line
    }

    /// Get the cursor column.
    pub fn cursor_col(&self) -> usize {
        self.cursor.col
    }

    /// Render the current editor state.
    pub fn render(&mut self) {
        if let Some(ref mut renderer) = self.renderer {
            renderer.render(&self.buffer, &self.cursor, self.scroll_y);
        }
    }

    /// Resize the rendering surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_height = height as f32;
        if let Some(ref mut renderer) = self.renderer {
            renderer.resize(width, height);
        }
    }

    /// Handle a key event from JavaScript. Returns true if the editor state changed.
    pub fn handle_key(&mut self, key: &str, ctrl: bool, shift: bool) -> bool {
        match key {
            "ArrowLeft" => {
                if ctrl {
                    self.move_word_left();
                } else {
                    self.move_left();
                }
                true
            }
            "ArrowRight" => {
                if ctrl {
                    self.move_word_right();
                } else {
                    self.move_right();
                }
                true
            }
            "ArrowUp" => {
                self.move_up();
                true
            }
            "ArrowDown" => {
                self.move_down();
                true
            }
            "Home" => {
                self.move_to_line_start();
                true
            }
            "End" => {
                self.move_to_line_end();
                true
            }
            "Backspace" => {
                self.delete_backward(1);
                true
            }
            "Delete" => {
                self.delete_forward(1);
                true
            }
            "Enter" => {
                self.insert_text("\n");
                true
            }
            "Tab" => {
                self.insert_text("    ");
                true
            }
            _ => {
                if !ctrl && key.len() == 1 {
                    self.insert_text(key);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Delete `count` characters after the cursor.
    pub fn delete_forward(&mut self, count: usize) {
        let len = self.buffer.len();
        let actual = count.min(len - self.cursor.offset);
        if actual == 0 {
            return;
        }
        self.buffer.delete(self.cursor.offset, actual);
        self.recalculate_cursor_position();
    }

    fn move_left(&mut self) {
        if self.cursor.offset > 0 {
            self.cursor.offset -= 1;
            // Skip back over multi-byte UTF-8
            let content = self.buffer.content();
            while self.cursor.offset > 0
                && !content.is_char_boundary(self.cursor.offset)
            {
                self.cursor.offset -= 1;
            }
            self.recalculate_cursor_position();
        }
    }

    fn move_right(&mut self) {
        let len = self.buffer.len();
        if self.cursor.offset < len {
            let content = self.buffer.content();
            let ch_len = content[self.cursor.offset..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor.offset += ch_len;
            self.recalculate_cursor_position();
        }
    }

    fn move_up(&mut self) {
        if self.cursor.line > 0 {
            let target_line = self.cursor.line - 1;
            let target_col = self.cursor.col.min(self.buffer.line_len(target_line));
            self.cursor.offset = self.buffer.line_start_offset(target_line) + target_col;
            self.recalculate_cursor_position();
        }
    }

    fn move_down(&mut self) {
        let max_line = self.buffer.line_count().saturating_sub(1);
        if self.cursor.line < max_line {
            let target_line = self.cursor.line + 1;
            let target_col = self.cursor.col.min(self.buffer.line_len(target_line));
            self.cursor.offset = self.buffer.line_start_offset(target_line) + target_col;
            self.recalculate_cursor_position();
        }
    }

    fn move_to_line_start(&mut self) {
        self.cursor.offset = self.buffer.line_start_offset(self.cursor.line);
        self.recalculate_cursor_position();
    }

    fn move_to_line_end(&mut self) {
        self.cursor.offset =
            self.buffer.line_start_offset(self.cursor.line) + self.buffer.line_len(self.cursor.line);
        self.recalculate_cursor_position();
    }

    fn move_word_left(&mut self) {
        if self.cursor.offset == 0 {
            return;
        }
        let content = self.buffer.content();
        let bytes = content.as_bytes();
        let mut pos = self.cursor.offset;
        // Skip whitespace
        while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        // Skip word characters
        while pos > 0 && !bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        self.cursor.offset = pos;
        self.recalculate_cursor_position();
    }

    fn move_word_right(&mut self) {
        let content = self.buffer.content();
        let len = content.len();
        let bytes = content.as_bytes();
        let mut pos = self.cursor.offset;
        // Skip word characters
        while pos < len && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.cursor.offset = pos;
        self.recalculate_cursor_position();
    }

    fn recalculate_cursor_position(&mut self) {
        let content = self.buffer.content();
        let before_cursor = &content[..self.cursor.offset.min(content.len())];
        self.cursor.line = before_cursor.matches('\n').count();
        self.cursor.col = before_cursor
            .rfind('\n')
            .map(|pos| before_cursor.len() - pos - 1)
            .unwrap_or(before_cursor.len());
    }
}
