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

/// Selection range (byte offsets). When anchor == cursor offset, nothing is selected.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub anchor: usize,
}

impl Selection {
    fn range(&self, cursor_offset: usize) -> Option<(usize, usize)> {
        if self.anchor == cursor_offset {
            None
        } else {
            Some((self.anchor.min(cursor_offset), self.anchor.max(cursor_offset)))
        }
    }
}

/// The main editor state, exposed to JavaScript.
#[wasm_bindgen]
pub struct Editor {
    buffer: TextBuffer,
    cursor: Cursor,
    selection: Selection,
    renderer: Option<Renderer>,
    scroll_y: f32,
    target_scroll_y: f32,
    viewport_height: f32,
    viewport_width: f32,
    scrollbar_dragging: bool,
    scrollbar_drag_offset: f32,
    scrollbar_opacity: f32,
    scrollbar_target_opacity: f32,
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
            selection: Selection { anchor: 0 },
            renderer: None,
            scroll_y: 0.0,
            target_scroll_y: 0.0,
            viewport_height: 0.0,
            viewport_width: 0.0,
            scrollbar_dragging: false,
            scrollbar_drag_offset: 0.0,
            scrollbar_opacity: 0.0,
            scrollbar_target_opacity: 0.0,
        }
    }

    pub async fn init_renderer(
        &mut self,
        canvas_id: &str,
        font_data: &[u8],
        device_pixel_ratio: f32,
    ) -> Result<(), JsValue> {
        let renderer = Renderer::new(canvas_id, font_data, device_pixel_ratio).await?;
        self.renderer = Some(renderer);
        Ok(())
    }

    pub fn set_content(&mut self, text: &str) {
        self.buffer = TextBuffer::new(text);
        self.cursor = Cursor { line: 0, col: 0, offset: 0 };
        self.selection = Selection { anchor: 0 };
    }

    pub fn get_content(&self) -> String {
        self.buffer.content()
    }

    pub fn insert_text(&mut self, text: &str) {
        self.delete_selection_if_any();
        self.buffer.insert(self.cursor.offset, text);
        self.cursor.offset += text.len();
        self.selection.anchor = self.cursor.offset;
        self.recalculate_cursor_position();
    }

    pub fn delete_backward(&mut self, count: usize) {
        if self.delete_selection_if_any() {
            return;
        }
        let actual = count.min(self.cursor.offset);
        if actual == 0 {
            return;
        }
        self.buffer.delete(self.cursor.offset - actual, actual);
        self.cursor.offset -= actual;
        self.selection.anchor = self.cursor.offset;
        self.recalculate_cursor_position();
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor.line
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor.col
    }

    pub fn render(&mut self) {
        if let Some(ref mut renderer) = self.renderer {
            let sel_range = self.selection.range(self.cursor.offset);
            renderer.render(&self.buffer, &self.cursor, self.scroll_y, sel_range, self.scrollbar_opacity);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_width = width as f32;
        self.viewport_height = height as f32;
        if let Some(ref mut renderer) = self.renderer {
            renderer.resize(width, height);
        }
    }

    /// Scroll by a pixel delta (positive = scroll down). Sets the target for smooth interpolation.
    pub fn scroll(&mut self, delta_y: f32) {
        let max_scroll = self.max_scroll();
        self.target_scroll_y = (self.target_scroll_y + delta_y).clamp(0.0, max_scroll);
    }

    /// Set whether the canvas is hovered (controls scrollbar fade).
    pub fn set_canvas_hovered(&mut self, hovered: bool) {
        self.scrollbar_target_opacity = if hovered { 1.0 } else { 0.0 };
    }

    /// Tick the smooth scroll interpolation and scrollbar fade. Returns true if still animating.
    pub fn tick(&mut self) -> bool {
        let mut animating = false;

        // Scroll interpolation
        let diff = self.target_scroll_y - self.scroll_y;
        if diff.abs() < 0.5 {
            self.scroll_y = self.target_scroll_y;
        } else {
            self.scroll_y += diff * 0.18;
            animating = true;
        }

        // Scrollbar opacity interpolation
        let opacity_diff = self.scrollbar_target_opacity - self.scrollbar_opacity;
        if opacity_diff.abs() < 0.01 {
            self.scrollbar_opacity = self.scrollbar_target_opacity;
        } else {
            self.scrollbar_opacity += opacity_diff * 0.12;
            animating = true;
        }

        animating
    }

    /// Whether smooth scroll animation is in progress.
    pub fn is_scrolling(&self) -> bool {
        (self.target_scroll_y - self.scroll_y).abs() > 0.5
    }

    /// Ensure the cursor is visible in the viewport, scrolling if needed.
    fn ensure_cursor_visible(&mut self) {
        let line_height = self.line_height();
        let cursor_top = self.cursor.line as f32 * line_height;
        let cursor_bottom = cursor_top + line_height;

        if cursor_top < self.target_scroll_y {
            self.target_scroll_y = cursor_top;
        } else if cursor_bottom > self.target_scroll_y + self.viewport_height {
            self.target_scroll_y = cursor_bottom - self.viewport_height;
        }
        // For keyboard-triggered scroll, snap immediately for responsiveness
        self.scroll_y = self.target_scroll_y;
    }

    fn max_scroll(&self) -> f32 {
        let line_height = self.line_height();
        (self.buffer.line_count() as f32 * line_height - self.viewport_height).max(0.0)
    }

    fn line_height(&self) -> f32 {
        self.renderer
            .as_ref()
            .map(|r| r.line_height())
            .unwrap_or(20.0)
    }

    /// Get scroll info for the scrollbar: (scroll_y, viewport_height, total_content_height).
    pub fn scroll_info(&self) -> Vec<f32> {
        let line_height = self.line_height();
        let total = self.buffer.line_count() as f32 * line_height;
        vec![self.scroll_y, self.viewport_height, total]
    }

    /// Set cursor from mouse click. If shift is held, extend selection.
    /// Returns true if the click was on the scrollbar.
    pub fn click(&mut self, pixel_x: f32, pixel_y: f32, shift: bool) -> bool {
        // Check if click is on scrollbar
        let scrollbar_width = 15.0;
        let scrollbar_x = self.viewport_width - scrollbar_width;
        let total_content = self.total_content_height();

        if pixel_x >= scrollbar_x && total_content > self.viewport_height {
            let thumb_ratio = self.viewport_height / total_content;
            let thumb_h = (thumb_ratio * self.viewport_height).max(20.0);
            let max_scroll = self.max_scroll();
            let scroll_ratio = if max_scroll > 0.0 { self.scroll_y / max_scroll } else { 0.0 };
            let thumb_y = scroll_ratio * (self.viewport_height - thumb_h);

            if pixel_y >= thumb_y && pixel_y <= thumb_y + thumb_h {
                // Clicked on thumb — start dragging with offset from thumb top
                self.scrollbar_dragging = true;
                self.scrollbar_drag_offset = pixel_y - thumb_y;
            } else {
                // Clicked on track — jump to that position
                let ratio = pixel_y / self.viewport_height;
                let target = ratio * max_scroll;
                self.scroll_y = target.clamp(0.0, max_scroll);
                self.target_scroll_y = self.scroll_y;
                self.scrollbar_dragging = true;
                self.scrollbar_drag_offset = thumb_h / 2.0;
            }
            return true;
        }

        let offset = self.pixel_to_offset(pixel_x, pixel_y);
        if !shift {
            self.selection.anchor = offset;
        }
        self.cursor.offset = offset;
        self.recalculate_cursor_position();
        false
    }

    /// Update during mouse drag — handles both text selection and scrollbar dragging.
    pub fn drag(&mut self, pixel_x: f32, pixel_y: f32) {
        if self.scrollbar_dragging {
            let total_content = self.total_content_height();
            let thumb_ratio = self.viewport_height / total_content;
            let thumb_h = (thumb_ratio * self.viewport_height).max(20.0);
            let max_scroll = self.max_scroll();
            let track_space = self.viewport_height - thumb_h;

            if track_space > 0.0 {
                let thumb_top = pixel_y - self.scrollbar_drag_offset;
                let ratio = thumb_top / track_space;
                let target = ratio * max_scroll;
                self.scroll_y = target.clamp(0.0, max_scroll);
                self.target_scroll_y = self.scroll_y;
            }
            return;
        }

        let offset = self.pixel_to_offset(pixel_x, pixel_y);
        self.cursor.offset = offset;
        self.recalculate_cursor_position();
    }

    /// End mouse drag.
    pub fn mouse_up(&mut self) {
        self.scrollbar_dragging = false;
    }

    fn total_content_height(&self) -> f32 {
        self.buffer.line_count() as f32 * self.line_height()
    }

    pub fn handle_key(&mut self, key: &str, ctrl: bool, shift: bool) -> bool {
        match key {
            "ArrowLeft" => {
                if ctrl {
                    self.move_word_left_sel(shift);
                } else {
                    self.move_left_sel(shift);
                }
                true
            }
            "ArrowRight" => {
                if ctrl {
                    self.move_word_right_sel(shift);
                } else {
                    self.move_right_sel(shift);
                }
                true
            }
            "ArrowUp" => {
                self.move_up_sel(shift);
                true
            }
            "ArrowDown" => {
                self.move_down_sel(shift);
                true
            }
            "Home" => {
                self.move_to_line_start_sel(shift);
                true
            }
            "End" => {
                self.move_to_line_end_sel(shift);
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
                if ctrl && key.to_lowercase() == "a" {
                    self.select_all();
                    return true;
                }
                if !ctrl && key.len() == 1 {
                    self.insert_text(key);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn delete_forward(&mut self, count: usize) {
        if self.delete_selection_if_any() {
            return;
        }
        let len = self.buffer.len();
        let actual = count.min(len - self.cursor.offset);
        if actual == 0 {
            return;
        }
        self.buffer.delete(self.cursor.offset, actual);
        self.selection.anchor = self.cursor.offset;
        self.recalculate_cursor_position();
    }

    /// Get selected text, or empty string if no selection.
    pub fn get_selection_text(&self) -> String {
        match self.selection.range(self.cursor.offset) {
            Some((start, end)) => {
                let content = self.buffer.content();
                content[start..end].to_string()
            }
            None => String::new(),
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection.anchor != self.cursor.offset
    }

    fn select_all(&mut self) {
        self.selection.anchor = 0;
        self.cursor.offset = self.buffer.len();
        self.recalculate_cursor_position();
    }

    /// Delete the current selection and collapse cursor. Returns true if there was a selection.
    fn delete_selection_if_any(&mut self) -> bool {
        if let Some((start, end)) = self.selection.range(self.cursor.offset) {
            self.buffer.delete(start, end - start);
            self.cursor.offset = start;
            self.selection.anchor = start;
            self.recalculate_cursor_position();
            true
        } else {
            false
        }
    }

    fn pixel_to_offset(&self, pixel_x: f32, pixel_y: f32) -> usize {
        let (cell_width, line_height, gutter_width) = match &self.renderer {
            Some(r) => (r.cell_width(), r.line_height(), r.gutter_width()),
            None => return 0,
        };

        let padding = 8.0;
        let text_start_x = gutter_width + padding;

        let line = ((pixel_y + self.scroll_y) / line_height).floor().max(0.0) as usize;
        let max_line = self.buffer.line_count().saturating_sub(1);
        let line = line.min(max_line);

        let col = if pixel_x > text_start_x {
            ((pixel_x - text_start_x) / cell_width).round() as usize
        } else {
            0
        };
        let col = col.min(self.buffer.line_len(line));

        self.buffer.line_start_offset(line) + col
    }

    // Movement helpers that optionally extend selection

    fn move_left_sel(&mut self, shift: bool) {
        if !shift && self.has_selection() {
            // Collapse to the start of selection
            let start = self.selection.anchor.min(self.cursor.offset);
            self.cursor.offset = start;
            self.selection.anchor = start;
            self.recalculate_cursor_position();
            return;
        }
        if self.cursor.offset > 0 {
            self.cursor.offset -= 1;
            let content = self.buffer.content();
            while self.cursor.offset > 0 && !content.is_char_boundary(self.cursor.offset) {
                self.cursor.offset -= 1;
            }
            if !shift {
                self.selection.anchor = self.cursor.offset;
            }
            self.recalculate_cursor_position();
        }
    }

    fn move_right_sel(&mut self, shift: bool) {
        if !shift && self.has_selection() {
            let end = self.selection.anchor.max(self.cursor.offset);
            self.cursor.offset = end;
            self.selection.anchor = end;
            self.recalculate_cursor_position();
            return;
        }
        let len = self.buffer.len();
        if self.cursor.offset < len {
            let content = self.buffer.content();
            let ch_len = content[self.cursor.offset..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            self.cursor.offset += ch_len;
            if !shift {
                self.selection.anchor = self.cursor.offset;
            }
            self.recalculate_cursor_position();
        }
    }

    fn move_up_sel(&mut self, shift: bool) {
        if self.cursor.line > 0 {
            let target_line = self.cursor.line - 1;
            let target_col = self.cursor.col.min(self.buffer.line_len(target_line));
            self.cursor.offset = self.buffer.line_start_offset(target_line) + target_col;
            if !shift {
                self.selection.anchor = self.cursor.offset;
            }
            self.recalculate_cursor_position();
        }
    }

    fn move_down_sel(&mut self, shift: bool) {
        let max_line = self.buffer.line_count().saturating_sub(1);
        if self.cursor.line < max_line {
            let target_line = self.cursor.line + 1;
            let target_col = self.cursor.col.min(self.buffer.line_len(target_line));
            self.cursor.offset = self.buffer.line_start_offset(target_line) + target_col;
            if !shift {
                self.selection.anchor = self.cursor.offset;
            }
            self.recalculate_cursor_position();
        }
    }

    fn move_to_line_start_sel(&mut self, shift: bool) {
        self.cursor.offset = self.buffer.line_start_offset(self.cursor.line);
        if !shift {
            self.selection.anchor = self.cursor.offset;
        }
        self.recalculate_cursor_position();
    }

    fn move_to_line_end_sel(&mut self, shift: bool) {
        self.cursor.offset =
            self.buffer.line_start_offset(self.cursor.line) + self.buffer.line_len(self.cursor.line);
        if !shift {
            self.selection.anchor = self.cursor.offset;
        }
        self.recalculate_cursor_position();
    }

    fn move_word_left_sel(&mut self, shift: bool) {
        if self.cursor.offset == 0 {
            return;
        }
        let content = self.buffer.content();
        let bytes = content.as_bytes();
        let mut pos = self.cursor.offset;
        while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        while pos > 0 && !bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        self.cursor.offset = pos;
        if !shift {
            self.selection.anchor = self.cursor.offset;
        }
        self.recalculate_cursor_position();
    }

    fn move_word_right_sel(&mut self, shift: bool) {
        let content = self.buffer.content();
        let len = content.len();
        let bytes = content.as_bytes();
        let mut pos = self.cursor.offset;
        while pos < len && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.cursor.offset = pos;
        if !shift {
            self.selection.anchor = self.cursor.offset;
        }
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
        self.ensure_cursor_visible();
    }
}
