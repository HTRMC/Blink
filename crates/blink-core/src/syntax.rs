#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Normal,
    Keyword,
    String,
    Comment,
    Number,
    Type,
    Function,
    Operator,
    Punctuation,
    Attribute,
    Property,
}

impl TokenKind {
    pub fn color(&self) -> [f32; 4] {
        match self {
            TokenKind::Normal      => [0.671, 0.698, 0.749, 1.0], // #abb2bf
            TokenKind::Keyword     => [0.776, 0.471, 0.867, 1.0], // #c678dd
            TokenKind::String      => [0.596, 0.765, 0.475, 1.0], // #98c379
            TokenKind::Comment     => [0.361, 0.388, 0.439, 1.0], // #5c6370
            TokenKind::Number      => [0.820, 0.604, 0.400, 1.0], // #d19a66
            TokenKind::Type        => [0.898, 0.753, 0.482, 1.0], // #e5c07b
            TokenKind::Function    => [0.380, 0.686, 0.937, 1.0], // #61afef
            TokenKind::Operator    => [0.337, 0.714, 0.761, 1.0], // #56b6c2
            TokenKind::Punctuation => [0.671, 0.698, 0.749, 1.0], // #abb2bf
            TokenKind::Attribute   => [0.776, 0.471, 0.867, 1.0], // #c678dd
            TokenKind::Property    => [0.878, 0.365, 0.365, 1.0], // #e06c75
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub start: usize,
    pub len: usize,
    pub kind: TokenKind,
}

pub struct Highlighter {
    lang: Lang,
    in_block_comment: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Lang {
    Rust,
    TypeScript,
    JavaScript,
    Css,
    Html,
    Json,
    Toml,
    Markdown,
    Plain,
}

fn lang_from_ext(ext: &str) -> Lang {
    match ext {
        "rs" => Lang::Rust,
        "ts" | "tsx" => Lang::TypeScript,
        "js" | "jsx" | "mjs" | "cjs" => Lang::JavaScript,
        "css" | "scss" | "less" => Lang::Css,
        "html" | "htm" | "svelte" | "vue" => Lang::Html,
        "json" | "jsonc" => Lang::Json,
        "toml" => Lang::Toml,
        "md" | "markdown" | "mdx" => Lang::Markdown,
        _ => Lang::Plain,
    }
}

fn is_keyword(word: &str, lang: Lang) -> bool {
    match lang {
        Lang::Rust => matches!(
            word,
            "as" | "async" | "await" | "break" | "const" | "continue" | "crate"
            | "dyn" | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if"
            | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut"
            | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct"
            | "super" | "trait" | "true" | "type" | "unsafe" | "use" | "where"
            | "while" | "yield"
        ),
        Lang::TypeScript | Lang::JavaScript => matches!(
            word,
            "abstract" | "as" | "async" | "await" | "break" | "case" | "catch"
            | "class" | "const" | "continue" | "debugger" | "default" | "delete"
            | "do" | "else" | "enum" | "export" | "extends" | "false" | "finally"
            | "for" | "from" | "function" | "get" | "if" | "implements" | "import"
            | "in" | "instanceof" | "interface" | "let" | "new" | "null" | "of"
            | "package" | "private" | "protected" | "public" | "return" | "set"
            | "static" | "super" | "switch" | "this" | "throw" | "true" | "try"
            | "type" | "typeof" | "undefined" | "var" | "void" | "while" | "with"
            | "yield"
        ),
        Lang::Css => matches!(
            word,
            "important" | "and" | "or" | "not" | "only" | "from" | "to"
        ),
        Lang::Html => matches!(
            word,
            "doctype" | "html" | "head" | "body" | "script" | "style" | "link"
            | "meta" | "title" | "div" | "span" | "class" | "id" | "src" | "href"
        ),
        Lang::Toml => false,
        Lang::Json => matches!(word, "true" | "false" | "null"),
        Lang::Markdown => false,
        Lang::Plain => false,
    }
}

fn is_type_like(word: &str) -> bool {
    let first = match word.chars().next() {
        Some(c) => c,
        None => return false,
    };
    first.is_uppercase() && word.len() > 1
}

impl Highlighter {
    pub fn new(ext: &str) -> Self {
        Highlighter {
            lang: lang_from_ext(ext),
            in_block_comment: false,
        }
    }

    pub fn reset(&mut self) {
        self.in_block_comment = false;
    }

    pub fn highlight_line(&mut self, line: &str) -> Vec<Token> {
        if self.lang == Lang::Plain {
            return vec![Token { start: 0, len: line.len(), kind: TokenKind::Normal }];
        }
        if self.lang == Lang::Markdown {
            return self.highlight_markdown(line);
        }
        if self.lang == Lang::Json {
            return Self::highlight_json(line);
        }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut tokens = Vec::new();
        let mut i = 0;

        // Continue block comment from previous line
        if self.in_block_comment {
            let end = find_block_comment_end(bytes, 0);
            match end {
                Some(pos) => {
                    tokens.push(Token { start: 0, len: pos, kind: TokenKind::Comment });
                    self.in_block_comment = false;
                    i = pos;
                }
                None => {
                    tokens.push(Token { start: 0, len: len, kind: TokenKind::Comment });
                    return tokens;
                }
            }
        }

        while i < len {
            let b = bytes[i];

            // Line comment
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                tokens.push(Token { start: i, len: len - i, kind: TokenKind::Comment });
                return tokens;
            }

            // Block comment start
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                match find_block_comment_end(bytes, i) {
                    Some(pos) => {
                        tokens.push(Token { start, len: pos - start, kind: TokenKind::Comment });
                        i = pos;
                    }
                    None => {
                        tokens.push(Token { start, len: len - start, kind: TokenKind::Comment });
                        self.in_block_comment = true;
                        return tokens;
                    }
                }
                continue;
            }

            // Hash comment (TOML)
            if b == b'#' && self.lang == Lang::Toml {
                tokens.push(Token { start: i, len: len - i, kind: TokenKind::Comment });
                return tokens;
            }

            // Strings
            if b == b'"' || b == b'\'' || b == b'`' {
                let quote = b;
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                tokens.push(Token { start, len: i - start, kind: TokenKind::String });
                continue;
            }

            // Rust attribute: #[...]  or #![...]
            if b == b'#' && self.lang == Lang::Rust && i + 1 < len && (bytes[i + 1] == b'[' || (bytes[i + 1] == b'!' && i + 2 < len && bytes[i + 2] == b'[')) {
                let start = i;
                while i < len && bytes[i] != b']' {
                    i += 1;
                }
                if i < len { i += 1; }
                tokens.push(Token { start, len: i - start, kind: TokenKind::Attribute });
                continue;
            }

            // Numbers
            if b.is_ascii_digit() || (b == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
                let start = i;
                // Hex
                if b == b'0' && i + 1 < len && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
                    i += 2;
                    while i < len && bytes[i].is_ascii_hexdigit() { i += 1; }
                } else {
                    while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_' || bytes[i] == b'e' || bytes[i] == b'E') {
                        i += 1;
                    }
                }
                // Suffix (e.g. f32, u64, px, em)
                while i < len && bytes[i].is_ascii_alphanumeric() { i += 1; }
                tokens.push(Token { start, len: i - start, kind: TokenKind::Number });
                continue;
            }

            // Identifiers / keywords
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];

                let kind = if is_keyword(word, self.lang) {
                    TokenKind::Keyword
                } else if is_type_like(word) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else if i < len && bytes[i] == b'!' && self.lang == Lang::Rust {
                    // Rust macro
                    TokenKind::Function
                } else {
                    TokenKind::Normal
                };

                tokens.push(Token { start, len: i - start, kind });
                continue;
            }

            // Operators
            if matches!(b, b'=' | b'+' | b'-' | b'*' | b'/' | b'%' | b'!' | b'<' | b'>' | b'&' | b'|' | b'^' | b'~' | b'?') {
                let start = i;
                i += 1;
                // Multi-char operators
                while i < len && matches!(bytes[i], b'=' | b'>' | b'&' | b'|') {
                    i += 1;
                    if i - start >= 3 { break; }
                }
                tokens.push(Token { start, len: i - start, kind: TokenKind::Operator });
                continue;
            }

            // Punctuation
            if matches!(b, b'(' | b')' | b'{' | b'}' | b'[' | b']' | b';' | b',' | b'.' | b':') {
                tokens.push(Token { start: i, len: 1, kind: TokenKind::Punctuation });
                i += 1;
                continue;
            }

            // Everything else (whitespace, etc.)
            let start = i;
            while i < len && !bytes[i].is_ascii_alphanumeric()
                && !matches!(bytes[i], b'_' | b'"' | b'\'' | b'`' | b'/' | b'#'
                    | b'=' | b'+' | b'-' | b'*' | b'%' | b'!' | b'<' | b'>'
                    | b'&' | b'|' | b'^' | b'~' | b'?' | b'(' | b')' | b'{'
                    | b'}' | b'[' | b']' | b';' | b',' | b'.' | b':')
            {
                i += 1;
            }
            if i > start {
                tokens.push(Token { start, len: i - start, kind: TokenKind::Normal });
            }
            if i == start {
                // Safety: advance at least one byte
                tokens.push(Token { start: i, len: 1, kind: TokenKind::Normal });
                i += 1;
            }
        }

        tokens
    }

    fn highlight_json(line: &str) -> Vec<Token> {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut tokens = Vec::new();
        let mut i = 0;

        // Detect if this line has a key by finding a string followed by ':'
        // We need to track whether the first string on the line is a key
        let mut found_key = false;

        while i < len {
            let b = bytes[i];

            // Whitespace
            if b == b' ' || b == b'\t' {
                i += 1;
                continue;
            }

            // Line comment (jsonc)
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                tokens.push(Token { start: i, len: len - i, kind: TokenKind::Comment });
                return tokens;
            }

            // Block comment
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                match find_block_comment_end(bytes, i) {
                    Some(pos) => {
                        tokens.push(Token { start, len: pos - start, kind: TokenKind::Comment });
                        i = pos;
                    }
                    None => {
                        tokens.push(Token { start, len: len - start, kind: TokenKind::Comment });
                        return tokens;
                    }
                }
                continue;
            }

            // Strings
            if b == b'"' {
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                // Check if this string is a key (followed by optional whitespace then ':')
                let mut j = i;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') { j += 1; }
                let is_key = j < len && bytes[j] == b':' && !found_key;
                if is_key {
                    found_key = true;
                    tokens.push(Token { start, len: i - start, kind: TokenKind::Property });
                } else {
                    tokens.push(Token { start, len: i - start, kind: TokenKind::String });
                }
                continue;
            }

            // Numbers (including negative)
            if b.is_ascii_digit() || (b == b'-' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
                let start = i;
                if b == b'-' { i += 1; }
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E' || bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                tokens.push(Token { start, len: i - start, kind: TokenKind::Number });
                continue;
            }

            // Keywords: true, false, null
            if b.is_ascii_alphabetic() {
                let start = i;
                while i < len && bytes[i].is_ascii_alphabetic() { i += 1; }
                let word = &line[start..i];
                let kind = if matches!(word, "true" | "false" | "null") {
                    TokenKind::Keyword
                } else {
                    TokenKind::Normal
                };
                tokens.push(Token { start, len: i - start, kind });
                continue;
            }

            // Punctuation: { } [ ] , :
            if matches!(b, b'{' | b'}' | b'[' | b']' | b',' | b':') {
                tokens.push(Token { start: i, len: 1, kind: TokenKind::Punctuation });
                i += 1;
                continue;
            }

            // Anything else
            tokens.push(Token { start: i, len: 1, kind: TokenKind::Normal });
            i += 1;
        }

        tokens
    }

    fn highlight_markdown(&mut self, line: &str) -> Vec<Token> {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut tokens = Vec::new();

        if len == 0 {
            return tokens;
        }

        // Fenced code block toggle
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            self.in_block_comment = !self.in_block_comment;
            tokens.push(Token { start: 0, len, kind: TokenKind::String });
            return tokens;
        }

        // Inside fenced code block
        if self.in_block_comment {
            tokens.push(Token { start: 0, len, kind: TokenKind::String });
            return tokens;
        }

        // Headings: # ## ### etc.
        if bytes[0] == b'#' {
            let mut lvl = 0;
            while lvl < len && bytes[lvl] == b'#' { lvl += 1; }
            if lvl <= 6 && (lvl == len || bytes[lvl] == b' ') {
                tokens.push(Token { start: 0, len: lvl, kind: TokenKind::Keyword });
                if lvl < len {
                    tokens.push(Token { start: lvl, len: len - lvl, kind: TokenKind::Function });
                }
                return tokens;
            }
        }

        // Horizontal rule: --- or *** or ___
        if len >= 3 && (trimmed.chars().all(|c| c == '-' || c == ' ')
            || trimmed.chars().all(|c| c == '*' || c == ' ')
            || trimmed.chars().all(|c| c == '_' || c == ' '))
            && trimmed.chars().filter(|c| !c.is_whitespace()).count() >= 3
        {
            tokens.push(Token { start: 0, len, kind: TokenKind::Comment });
            return tokens;
        }

        // Blockquote: > text
        if bytes[0] == b'>' {
            tokens.push(Token { start: 0, len: 1, kind: TokenKind::Keyword });
            if len > 1 {
                tokens.push(Token { start: 1, len: len - 1, kind: TokenKind::Comment });
            }
            return tokens;
        }

        // List items: - or * or + or 1. 2. etc.
        let list_prefix = detect_list_prefix(trimmed);
        let indent = len - trimmed.len();

        // Inline formatting
        let mut i = 0;
        if list_prefix > 0 {
            tokens.push(Token { start: 0, len: indent + list_prefix, kind: TokenKind::Keyword });
            i = indent + list_prefix;
        }

        while i < len {
            let b = bytes[i];

            // Inline code: `...`
            if b == b'`' {
                let start = i;
                i += 1;
                while i < len && bytes[i] != b'`' { i += 1; }
                if i < len { i += 1; }
                tokens.push(Token { start, len: i - start, kind: TokenKind::String });
                continue;
            }

            // Bold: **text** or __text__
            if (b == b'*' || b == b'_') && i + 1 < len && bytes[i + 1] == b {
                let marker = b;
                let start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == marker && bytes[i + 1] == marker) { i += 1; }
                if i + 1 < len { i += 2; } else { i = len; }
                tokens.push(Token { start, len: i - start, kind: TokenKind::Type });
                continue;
            }

            // Italic: *text* or _text_
            if (b == b'*' || b == b'_') && i + 1 < len && bytes[i + 1] != b' ' {
                let marker = b;
                let start = i;
                i += 1;
                while i < len && bytes[i] != marker { i += 1; }
                if i < len { i += 1; }
                tokens.push(Token { start, len: i - start, kind: TokenKind::Normal });
                continue;
            }

            // Links: [text](url)
            if b == b'[' {
                let start = i;
                i += 1;
                while i < len && bytes[i] != b']' { i += 1; }
                if i < len { i += 1; }
                let bracket_end = i;
                if i < len && bytes[i] == b'(' {
                    i += 1;
                    while i < len && bytes[i] != b')' { i += 1; }
                    if i < len { i += 1; }
                    tokens.push(Token { start, len: bracket_end - start, kind: TokenKind::Function });
                    tokens.push(Token { start: bracket_end, len: i - bracket_end, kind: TokenKind::String });
                } else {
                    tokens.push(Token { start, len: i - start, kind: TokenKind::Function });
                }
                continue;
            }

            // Normal text
            let start = i;
            while i < len && !matches!(bytes[i], b'`' | b'*' | b'_' | b'[') { i += 1; }
            if i > start {
                tokens.push(Token { start, len: i - start, kind: TokenKind::Normal });
            }
        }

        tokens
    }
}

fn detect_list_prefix(trimmed: &str) -> usize {
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() { return 0; }
    // Unordered: - item, * item, + item
    if matches!(bytes[0], b'-' | b'*' | b'+') && bytes.len() > 1 && bytes[1] == b' ' {
        return 2;
    }
    // Ordered: 1. item, 12. item
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
    if i > 0 && i < bytes.len() && bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
        return i + 2;
    }
    0
}

fn find_block_comment_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

/// Build a per-character color array for a line, given tokens.
pub fn colors_for_line(line: &str, tokens: &[Token]) -> Vec<[f32; 4]> {
    let len = line.len();
    let default_color = TokenKind::Normal.color();
    let mut colors = vec![default_color; len];
    for tok in tokens {
        let end = (tok.start + tok.len).min(len);
        for c in &mut colors[tok.start..end] {
            *c = tok.kind.color();
        }
    }
    colors
}
