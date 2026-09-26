//! JavaScript string semantics that affect output bytes.
//!
//! Offsets in this crate are UTF-8 byte offsets, but a few values must be counted exactly like the
//! TypeScript implementation counts them (UTF-16 code units): wrap widths and columns, and the length
//! limits of hints and structure labels.

/// Length in UTF-16 code units (JavaScript's `String.prototype.length`).
pub fn u16len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// JavaScript's `\s` / `String.prototype.trim` whitespace.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{0B}'
            | '\u{0C}'
            | '\r'
            | ' '
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

pub fn trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

pub fn trim_start(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

pub fn trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_space)
}

/// Leading whitespace (`/^\s*/.exec(s)[0]`).
pub fn leading_ws(s: &str) -> &str {
    &s[..s.len() - trim_start(s).len()]
}

/// Trailing whitespace (`/\s*$/.exec(s)[0]`).
pub fn trailing_ws(s: &str) -> &str {
    &s[trim_end(s).len()..]
}

/// Leading spaces and tabs (`/^[ \t]*/.exec(s)[0]`).
pub fn leading_blank(s: &str) -> &str {
    let n = s.bytes().take_while(|b| *b == b' ' || *b == b'\t').count();
    &s[..n]
}

/// `s.replace(/\s+/g, ' ')`.
pub fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for c in s.chars() {
        if is_js_space(c) {
            if !in_ws {
                out.push(' ');
            }
            in_ws = true;
        } else {
            out.push(c);
            in_ws = false;
        }
    }
    out
}

/// Longest prefix of at most `n` UTF-16 code units (JavaScript `s.slice(0, n)`), never splitting a
/// character.
pub fn prefix_u16(s: &str, n: usize) -> &str {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        units += c.len_utf16();
        if units > n {
            return &s[..i];
        }
    }
    s
}

/// `JSON.stringify(s)` for a string.
pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Compares strings like JavaScript's default `Array.prototype.sort` (by UTF-16 code units).
pub fn cmp_u16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Offset of the start of the line containing `offset` (`src.lastIndexOf('\n', offset - 1) + 1`).
pub fn line_start(src: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    src[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Next `needle` at or after `from` (`src.indexOf(needle, from)`).
pub fn index_of(src: &str, needle: &str, from: usize) -> Option<usize> {
    if from > src.len() {
        return None;
    }
    src[from..].find(needle).map(|i| i + from)
}

/// Clamps `i` down to a character boundary.
pub fn floor_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_lengths() {
        assert_eq!(u16len("abc"), 3);
        assert_eq!(u16len("é"), 1);
        assert_eq!(u16len("😀"), 2);
        assert_eq!(prefix_u16("a😀b", 2), "a");
        assert_eq!(prefix_u16("a😀b", 3), "a😀");
    }

    #[test]
    fn json_matches_js() {
        assert_eq!(json_string("a\"b\\c\n\u{1}é"), r#""a\"b\\c\n\u0001é""#);
    }

    #[test]
    fn whitespace() {
        assert_eq!(collapse_ws("a \n\t b"), "a b");
        assert_eq!(trim("\u{FEFF} a "), "a");
        assert_eq!(line_start("ab\ncd", 4), 3);
        assert_eq!(line_start("ab\ncd", 3), 3);
        assert_eq!(line_start("ab\ncd", 2), 0);
    }
}
