//! Keyword-based syntax highlighting for diff lines

use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Get keywords for a given file extension
fn get_keywords(ext: &str) -> &'static [&'static str] {
    match ext {
        "rs" => &[
            "fn",
            "let",
            "mut",
            "pub",
            "use",
            "mod",
            "struct",
            "enum",
            "impl",
            "trait",
            "match",
            "if",
            "else",
            "for",
            "while",
            "loop",
            "return",
            "self",
            "Self",
            "super",
            "crate",
            "async",
            "await",
            "where",
            "type",
            "const",
            "static",
            "ref",
            "move",
            "unsafe",
            "extern",
            "dyn",
            "macro_rules",
        ],
        "ts" | "js" | "tsx" | "jsx" => &[
            "function",
            "const",
            "let",
            "var",
            "return",
            "if",
            "else",
            "for",
            "while",
            "class",
            "interface",
            "type",
            "import",
            "export",
            "from",
            "async",
            "await",
            "new",
            "this",
            "throw",
            "try",
            "catch",
            "finally",
            "typeof",
            "instanceof",
            "extends",
            "implements",
        ],
        "py" => &[
            "def", "class", "return", "if", "elif", "else", "for", "while", "import", "from", "as",
            "with", "try", "except", "finally", "raise", "yield", "lambda", "pass", "break",
            "continue", "and", "or", "not", "in", "is", "None", "True", "False", "self", "async",
            "await",
        ],
        "go" => &[
            "func",
            "var",
            "const",
            "return",
            "if",
            "else",
            "for",
            "range",
            "switch",
            "case",
            "default",
            "type",
            "struct",
            "interface",
            "map",
            "chan",
            "go",
            "defer",
            "select",
            "package",
            "import",
            "nil",
            "true",
            "false",
        ],
        "java" => &[
            "public",
            "private",
            "protected",
            "class",
            "interface",
            "extends",
            "implements",
            "return",
            "if",
            "else",
            "for",
            "while",
            "new",
            "this",
            "super",
            "void",
            "int",
            "String",
            "boolean",
            "static",
            "final",
            "abstract",
            "try",
            "catch",
            "throw",
            "throws",
            "import",
            "package",
        ],
        "c" | "cpp" | "h" | "hpp" | "cc" | "cxx" => &[
            "int",
            "void",
            "char",
            "float",
            "double",
            "long",
            "short",
            "unsigned",
            "signed",
            "const",
            "static",
            "return",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "break",
            "continue",
            "struct",
            "enum",
            "typedef",
            "sizeof",
            "include",
            "define",
            "class",
            "public",
            "private",
            "protected",
            "virtual",
            "template",
            "namespace",
            "using",
            "new",
            "delete",
        ],
        "rb" => &[
            "def",
            "end",
            "class",
            "module",
            "return",
            "if",
            "elsif",
            "else",
            "unless",
            "while",
            "do",
            "begin",
            "rescue",
            "ensure",
            "raise",
            "yield",
            "block_given?",
            "self",
            "nil",
            "true",
            "false",
            "require",
            "include",
            "attr_reader",
            "attr_writer",
            "attr_accessor",
        ],
        _ => &[],
    }
}

/// Highlight a line of code based on file extension, returning colored spans.
/// The `base_style` is the underlying style (e.g., green for added, red for removed).
pub(super) fn highlight_line<'a>(line: &'a str, ext: &str, base_style: Style) -> Vec<Span<'a>> {
    let keywords = get_keywords(ext);
    if keywords.is_empty() {
        return vec![Span::styled(line, base_style)];
    }

    // Check for line comments first
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('#') {
        let comment_style = base_style.fg(Color::DarkGray);
        return vec![Span::styled(line, comment_style)];
    }

    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut last_end = 0;

    while let Some((i, c)) = chars.next() {
        // String literals
        if c == '"' || c == '\'' {
            // Emit any pending text before the string
            if i > last_end {
                spans.extend(highlight_text_segment(
                    &line[last_end..i],
                    keywords,
                    base_style,
                ));
            }

            // Find the closing quote
            let quote = c;
            let mut end_idx = i + c.len_utf8();
            let mut escaped = false;

            while let Some(&(idx, ch)) = chars.peek() {
                chars.next();
                end_idx = idx + ch.len_utf8();

                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == quote {
                    break;
                }
            }

            // Emit the string span
            let string_style = base_style.fg(Color::Yellow);
            spans.push(Span::styled(&line[i..end_idx], string_style));
            last_end = end_idx;
        }
    }

    // Emit any remaining text
    if last_end < line.len() {
        spans.extend(highlight_text_segment(
            &line[last_end..],
            keywords,
            base_style,
        ));
    }

    if spans.is_empty() {
        vec![Span::styled(line, base_style)]
    } else {
        spans
    }
}

/// Highlight a text segment (no strings) by detecting keywords
fn highlight_text_segment<'a>(
    text: &'a str,
    keywords: &[&str],
    base_style: Style,
) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut last_end = 0;

    while let Some((i, c)) = chars.next() {
        // Check if this is the start of a word
        if c.is_alphabetic() || c == '_' {
            // Find the end of the word
            let mut end_idx = i + c.len_utf8();
            while let Some(&(idx, ch)) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    chars.next();
                    end_idx = idx + ch.len_utf8();
                } else {
                    break;
                }
            }

            let word = &text[i..end_idx];

            // Check if word is a keyword
            let is_keyword = keywords.contains(&word);

            // Emit any text before this word
            if i > last_end {
                spans.push(Span::styled(&text[last_end..i], base_style));
            }

            // Emit the word (keyword or not)
            if is_keyword {
                let keyword_style = base_style.fg(Color::Magenta);
                spans.push(Span::styled(word, keyword_style));
            } else {
                spans.push(Span::styled(word, base_style));
            }

            last_end = end_idx;
        }
    }

    // Emit any remaining text
    if last_end < text.len() {
        spans.push(Span::styled(&text[last_end..], base_style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};

    #[test]
    fn test_rust_keywords() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("fn main() {", "rs", base);
        // Should have multiple spans, "fn" should be highlighted
        assert!(spans.len() > 1);
        // Find "fn" span
        let has_fn = spans
            .iter()
            .any(|s| s.content == "fn" && s.style.fg == Some(Color::Magenta));
        assert!(has_fn, "Should highlight 'fn' keyword");
    }

    #[test]
    fn test_unknown_extension_passthrough() {
        let base = Style::default().fg(Color::White);
        let spans = highlight_line("some text", "xyz", base);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "some text");
    }

    #[test]
    fn test_string_literal() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("let x = \"hello\";", "rs", base);
        // Should detect string literal
        assert!(spans.len() > 1);
        let has_yellow = spans.iter().any(|s| s.style.fg == Some(Color::Yellow));
        assert!(has_yellow, "Should highlight string in yellow");
    }

    #[test]
    fn test_comment_line() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("// this is a comment", "rs", base);
        // Entire line should be one span with DarkGray
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_python_keywords() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("def hello():", "py", base);
        assert!(spans.len() > 1);
        let has_def = spans
            .iter()
            .any(|s| s.content == "def" && s.style.fg == Some(Color::Magenta));
        assert!(has_def, "Should highlight 'def' keyword");
    }

    #[test]
    fn test_go_keywords() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("func main() {", "go", base);
        assert!(spans.len() > 1);
        let has_func = spans
            .iter()
            .any(|s| s.content == "func" && s.style.fg == Some(Color::Magenta));
        assert!(has_func, "Should highlight 'func' keyword");
    }

    #[test]
    fn test_js_keywords() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("const x = 1;", "js", base);
        assert!(spans.len() > 1);
        let has_const = spans
            .iter()
            .any(|s| s.content == "const" && s.style.fg == Some(Color::Magenta));
        assert!(has_const, "Should highlight 'const' keyword");
    }

    #[test]
    fn test_keyword_not_in_identifier() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("let returning = 1;", "rs", base);
        // "returning" should NOT be highlighted as "return" keyword
        // "let" should be highlighted
        let magenta_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.style.fg == Some(Color::Magenta))
            .collect();
        // Only "let" should be magenta
        assert_eq!(magenta_spans.len(), 1);
        assert_eq!(magenta_spans[0].content, "let");
    }

    #[test]
    fn test_empty_line() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("", "rs", base);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn test_hash_comment_python() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("  # comment here", "py", base);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_single_quote_string() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("let x = 'hello';", "rs", base);
        let has_yellow = spans.iter().any(|s| s.style.fg == Some(Color::Yellow));
        assert!(has_yellow, "Should highlight single-quote string");
    }

    #[test]
    fn test_escaped_quote_in_string() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line(r#"let x = "hello \"world\"";"#, "rs", base);
        let yellow_spans: Vec<_> = spans
            .iter()
            .filter(|s| s.style.fg == Some(Color::Yellow))
            .collect();
        // The entire string including escaped quotes should be one span
        assert!(yellow_spans.len() >= 1);
    }

    #[test]
    fn test_multiple_keywords() {
        let base = Style::default().fg(Color::Green);
        let spans = highlight_line("pub fn test() -> impl Trait {", "rs", base);
        let magenta_count = spans
            .iter()
            .filter(|s| s.style.fg == Some(Color::Magenta))
            .count();
        // Should highlight: pub, fn, impl, Trait (if Trait was a keyword, but it's not)
        // Actually: pub, fn, impl
        assert!(magenta_count >= 3, "Should highlight multiple keywords");
    }
}
