use serde::{Deserialize, Serialize};

/// A piece in the piece table, referencing either the original or add buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Piece {
    source: PieceSource,
    start: usize,
    length: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
enum PieceSource {
    Original,
    Add,
}

/// Piece table text buffer for efficient editing.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    original: String,
    add: String,
    pieces: Vec<Piece>,
}

impl TextBuffer {
    pub fn new(initial_text: &str) -> Self {
        let pieces = if initial_text.is_empty() {
            vec![]
        } else {
            vec![Piece {
                source: PieceSource::Original,
                start: 0,
                length: initial_text.len(),
            }]
        };

        TextBuffer {
            original: initial_text.to_string(),
            add: String::new(),
            pieces,
        }
    }

    /// Get the full text content by walking the piece table.
    pub fn content(&self) -> String {
        let mut result = String::new();
        for piece in &self.pieces {
            let source = match piece.source {
                PieceSource::Original => &self.original,
                PieceSource::Add => &self.add,
            };
            result.push_str(&source[piece.start..piece.start + piece.length]);
        }
        result
    }

    /// Total length of the text.
    pub fn len(&self) -> usize {
        self.pieces.iter().map(|p| p.length).sum()
    }


    /// Insert text at the given byte offset.
    pub fn insert(&mut self, offset: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        let add_start = self.add.len();
        self.add.push_str(text);

        let new_piece = Piece {
            source: PieceSource::Add,
            start: add_start,
            length: text.len(),
        };

        if self.pieces.is_empty() {
            self.pieces.push(new_piece);
            return;
        }

        // Find the piece and split point
        let mut pos = 0;
        for i in 0..self.pieces.len() {
            let piece_end = pos + self.pieces[i].length;

            if offset <= piece_end {
                if offset == pos {
                    // Insert before this piece
                    self.pieces.insert(i, new_piece);
                    return;
                } else if offset == piece_end {
                    // Insert after this piece
                    self.pieces.insert(i + 1, new_piece);
                    return;
                } else {
                    // Split this piece
                    let split_offset = offset - pos;
                    let original_piece = self.pieces[i].clone();

                    self.pieces[i] = Piece {
                        source: original_piece.source,
                        start: original_piece.start,
                        length: split_offset,
                    };

                    let right_piece = Piece {
                        source: original_piece.source,
                        start: original_piece.start + split_offset,
                        length: original_piece.length - split_offset,
                    };

                    self.pieces.insert(i + 1, new_piece);
                    self.pieces.insert(i + 2, right_piece);
                    return;
                }
            }

            pos = piece_end;
        }

        // Append at the end
        self.pieces.push(new_piece);
    }

    /// Delete `count` bytes starting at `offset`.
    pub fn delete(&mut self, offset: usize, count: usize) {
        if count == 0 {
            return;
        }

        let mut new_pieces = Vec::new();
        let mut pos = 0;
        let delete_end = offset + count;

        for piece in &self.pieces {
            let piece_end = pos + piece.length;

            if piece_end <= offset || pos >= delete_end {
                // Entirely outside the delete range, keep as-is
                new_pieces.push(piece.clone());
            } else {
                // This piece overlaps the delete range
                if pos < offset {
                    // Keep the left part
                    new_pieces.push(Piece {
                        source: piece.source,
                        start: piece.start,
                        length: offset - pos,
                    });
                }
                if piece_end > delete_end {
                    // Keep the right part
                    let skip = delete_end - pos;
                    new_pieces.push(Piece {
                        source: piece.source,
                        start: piece.start + skip,
                        length: piece.length - skip,
                    });
                }
            }

            pos = piece_end;
        }

        self.pieces = new_pieces;
    }

    /// Get all lines as a vector of strings.
    pub fn lines(&self) -> Vec<String> {
        self.content().lines().map(|l| l.to_string()).collect()
    }

    /// Get the byte offset where a given line starts.
    pub fn line_start_offset(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        let content = self.content();
        let mut count = 0;
        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                count += 1;
                if count == line {
                    return i + 1;
                }
            }
        }
        content.len()
    }

    /// Get the length of a specific line (excluding newline).
    pub fn line_len(&self, line: usize) -> usize {
        let lines = self.lines();
        lines.get(line).map(|l| l.len()).unwrap_or(0)
    }

    /// Count the number of lines.
    pub fn line_count(&self) -> usize {
        let content = self.content();
        if content.is_empty() {
            return 1;
        }
        content.lines().count() + if content.ends_with('\n') { 1 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = TextBuffer::new("hello world");
        assert_eq!(buf.content(), "hello world");
        assert_eq!(buf.len(), 11);
    }

    #[test]
    fn test_insert() {
        let mut buf = TextBuffer::new("hllo");
        buf.insert(1, "e");
        assert_eq!(buf.content(), "hello");
    }

    #[test]
    fn test_delete() {
        let mut buf = TextBuffer::new("hello world");
        buf.delete(5, 6);
        assert_eq!(buf.content(), "hello");
    }

    #[test]
    fn test_empty_buffer() {
        let buf = TextBuffer::new("");
        assert!(buf.is_empty());
        assert_eq!(buf.line_count(), 1);
    }
}
