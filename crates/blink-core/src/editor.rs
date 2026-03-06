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

    /// Initialize the WebGPU renderer on the given canvas.
    pub async fn init_renderer(&mut self, canvas_id: &str) -> Result<(), JsValue> {
        let renderer = Renderer::new(canvas_id).await?;
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
