/// A replacement range and the resulting selection, all measured in UTF-16
/// code units. Selection offsets refer to the text *after* applying the edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub start: u32,
    pub end: u32,
    pub replacement: String,
    pub selection_start: u32,
    pub selection_end: u32,
}

pub(crate) fn units(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

// Browser offsets can land inside a surrogate pair. Snap backward rather
// than slicing a Rust string at an invalid character boundary.
pub(crate) fn byte_offset(text: &str, offset: u32) -> usize {
    let mut position = 0u64;
    for (byte, ch) in text.char_indices() {
        let next = position + ch.len_utf16() as u64;
        if next > u64::from(offset) {
            return byte;
        }
        position = next;
    }
    text.len()
}

pub(crate) fn range(text: &str, start: u32, end: u32) -> (usize, usize) {
    (
        byte_offset(text, start.min(end)),
        byte_offset(text, start.max(end)),
    )
}

pub(crate) fn line_start(text: &str, byte: usize) -> usize {
    text[..byte].rfind('\n').map_or(0, |index| index + 1)
}

/// Replace a selection with a newline and the current line's leading spaces
/// and tabs. CRLF is retained when it is the document's first line ending.
/// No language-specific indentation is inferred.
pub fn newline(text: &str, start: u32, end: u32) -> TextEdit {
    let (start, end) = range(text, start, end);
    let prefix = &text[line_start(text, start)..start];
    let whitespace: String = prefix
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .collect();
    let ending = if text
        .find('\n')
        .is_some_and(|index| index > 0 && text.as_bytes()[index - 1] == b'\r')
    {
        "\r\n"
    } else {
        "\n"
    };
    let replacement = format!("{ending}{whitespace}");
    let start = units(&text[..start]);
    let cursor = start.saturating_add(units(&replacement));
    TextEdit {
        start,
        end: units(&text[..end]),
        replacement,
        selection_start: cursor,
        selection_end: cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_preserves_indent_and_crlf() {
        let edit = newline("\t  😀x\r\nnext", 6, 6);
        assert_eq!(edit.replacement, "\r\n\t  ");
        assert_eq!((edit.start, edit.selection_start), (6, 11));
    }

    #[test]
    fn normalizes_unicode_ranges() {
        let edit = newline("a😀b", 99, 2);
        assert_eq!((edit.start, edit.end), (1, 4));
        assert_eq!(edit.selection_start, 2);
        assert_eq!(newline("", 10, 0).replacement, "\n");
    }

    #[test]
    fn indentation_is_limited_to_prefix_before_cursor() {
        assert_eq!(newline("    abc", 2, 7).replacement, "\n  ");
    }
}
