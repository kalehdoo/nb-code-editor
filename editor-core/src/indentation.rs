use crate::text_operations::{line_start, range, units, TextEdit};

/// Indent selected lines, or insert spaces at a collapsed cursor. Outdent
/// removes up to `size` leading spaces or one leading tab from each line.
/// A selection ending at the next line's start does not include that line.
/// Reversed selections are returned in ascending order; invalid offsets are
/// clamped and surrogate-interior offsets snap backward.
pub fn indent(text: &str, start: u32, end: u32, size: usize, outdent: bool) -> TextEdit {
    let (start, end) = range(text, start, end);
    let selection_start = units(&text[..start]);
    let selection_end = units(&text[..end]);
    if start == end && !outdent {
        let replacement = " ".repeat(size);
        let cursor = selection_start.saturating_add(units(&replacement));
        return TextEdit {
            start: selection_start,
            end: selection_end,
            replacement,
            selection_start: cursor,
            selection_end: cursor,
        };
    }
    let first = line_start(text, start);
    let last = if end > start && text.as_bytes()[end - 1] == b'\n' {
        end
    } else {
        text[end..]
            .find('\n')
            .map_or(text.len(), |offset| end + offset)
    };
    let mut replacement = String::new();
    let mut mapped_start = i64::from(selection_start);
    let mut mapped_end = i64::from(selection_end);
    let mut position = units(&text[..first]);
    for line in text[first..last].split_inclusive('\n') {
        let removed = if outdent && size > 0 {
            if line.starts_with('\t') {
                1
            } else {
                line.bytes().take(size).take_while(|ch| *ch == b' ').count()
            }
        } else {
            0
        };
        let added = if outdent { 0 } else { size };
        replacement.push_str(&" ".repeat(added));
        replacement.push_str(&line[removed..]);
        for (original, mapped) in [
            (selection_start, &mut mapped_start),
            (selection_end, &mut mapped_end),
        ] {
            if original >= position {
                *mapped += added as i64 - i64::from((original - position).min(removed as u32));
            }
        }
        position = position.saturating_add(units(line));
    }
    // An empty final line still participates when the cursor is on that line.
    if first == last && !outdent {
        replacement.push_str(&" ".repeat(size));
    }
    TextEdit {
        start: units(&text[..first]),
        end: units(&text[..last]),
        replacement,
        selection_start: mapped_start.clamp(0, i64::from(u32::MAX)) as u32,
        selection_end: mapped_end.clamp(0, i64::from(u32::MAX)) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_unicode_cursor() {
        let edit = indent("😀x", 2, 2, 2, false);
        assert_eq!((edit.start, edit.end, edit.selection_start), (2, 2, 4));
        assert_eq!(edit.replacement, "  ");
    }

    #[test]
    fn multiline_excludes_line_at_end_boundary() {
        let edit = indent("😀a\r\nb\r\nc", 0, 8, 2, false);
        assert_eq!(edit.replacement, "  😀a\r\n  b\r\n");
        assert_eq!((edit.selection_start, edit.selection_end), (2, 12));
    }

    #[test]
    fn outdent_clamps_selection_inside_removed_indent() {
        let edit = indent("    😀\n\tx\n z", 1, 12, 2, true);
        assert_eq!(edit.replacement, "  😀\nx\nz");
        assert_eq!((edit.selection_start, edit.selection_end), (0, 8));
    }

    #[test]
    fn reversed_and_oversized_ranges() {
        let edit = indent("a\nb", 100, 0, 1, false);
        assert_eq!(edit.replacement, " a\n b");
        assert_eq!((edit.selection_start, edit.selection_end), (1, 5));
    }

    #[test]
    fn zero_size_and_empty_outdent_are_noops() {
        let edit = indent("  a", 1, 1, 0, true);
        assert_eq!(edit.replacement, "  a");
        assert_eq!(edit.selection_start, 1);
        assert_eq!(indent("", 0, 0, 2, true).replacement, "");
    }
}
