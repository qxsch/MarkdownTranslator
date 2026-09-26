//! Greedy re-wrapping of translated blocks to the source width. Port of `wrap.ts`; widths are counted in
//! UTF-16 code units like the TypeScript implementation.

use crate::jsstr::u16len;
use crate::mask::masking::Piece;
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::WrapSpec;

struct Word {
    text: String,
    space_before: bool,
}

rx!(UNSAFE_LIST_OR_HEADING, r"^(?:[-+*]|\d{1,9}[.)]|#{1,6})$");
rx!(UNSAFE_BLOCK_START, r"^(?:>|`{3,}|~{3,}|<[A-Za-z!/?]|\|)");
rx!(UNSAFE_RULE, r"^(?:=+|-+|_+|\*+)$");

/// A word at the start of a line must not turn into block syntax (list item, heading, quote, fence, HTML block...).
pub fn safe_line_start(word: &str) -> bool {
    !(UNSAFE_LIST_OR_HEADING().test(word) || UNSAFE_BLOCK_START().test(word) || UNSAFE_RULE().test(word))
}

/// `s.split(/( +)/)`: alternating non-space parts and space runs (runs of U+0020 only).
fn split_spaces(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    let mut start = 0;
    while i < b.len() {
        if b[i] == b' ' {
            out.push(&s[start..i]);
            let run = i;
            while i < b.len() && b[i] == b' ' {
                i += 1;
            }
            out.push(&s[run..i]);
            start = i;
        } else {
            i += 1;
        }
    }
    out.push(&s[start..]);
    out
}

fn to_words(pieces: &[Piece]) -> Vec<Word> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut space_before = false;
    let mut pending_space = false;
    fn flush(words: &mut Vec<Word>, cur: &mut String, space_before: bool) {
        if !cur.is_empty() {
            words.push(Word { text: std::mem::take(cur), space_before });
        }
    }
    for p in pieces {
        if p.atom {
            if pending_space {
                flush(&mut words, &mut cur, space_before);
                space_before = true;
                pending_space = false;
            }
            cur.push_str(&p.s);
            continue;
        }
        for part in split_spaces(&p.s) {
            if part.is_empty() {
                continue;
            }
            if part.starts_with(' ') {
                if !cur.is_empty() {
                    flush(&mut words, &mut cur, space_before);
                    space_before = false;
                }
                pending_space = true;
            } else {
                if pending_space {
                    flush(&mut words, &mut cur, space_before);
                    space_before = true;
                    pending_space = false;
                }
                cur.push_str(part);
            }
        }
    }
    flush(&mut words, &mut cur, space_before);
    if pending_space {
        words.push(Word { text: String::new(), space_before: true });
    }
    words
}

pub fn join_pieces(pieces: &[Piece]) -> String {
    pieces.iter().map(|p| p.s.as_str()).collect()
}

/// Greedy re-wrap to the source width; placeholders (inline code, link targets, breaks) are never split.
pub fn wrap_pieces(pieces: &[Piece], spec: &WrapSpec) -> String {
    let words = to_words(pieces);
    let mut out = String::new();
    let mut col = spec.first_column;
    let mut line_has_content = false;
    let prefix_len = u16len(&spec.prefix);
    for w in &words {
        let sp = if w.space_before { " " } else { "" };
        let nl = w.text.find('\n');
        let first_len = match nl {
            Some(i) => u16len(&w.text[..i]),
            None => u16len(&w.text),
        };
        if line_has_content && !sp.is_empty() && !w.text.is_empty() && col + 1 + first_len > spec.width && safe_line_start(&w.text) {
            out.push_str(&spec.eol);
            out.push_str(&spec.prefix);
            out.push_str(&w.text);
            col = prefix_len;
        } else {
            out.push_str(sp);
            out.push_str(&w.text);
            col += sp.len();
        }
        match nl {
            None => {
                col += u16len(&w.text);
                line_has_content = line_has_content || !w.text.is_empty();
            }
            Some(_) => {
                let tail = &w.text[w.text.rfind('\n').unwrap() + 1..];
                col = u16len(tail);
                // After a hard break the next word continues right after the break's prefix.
                line_has_content = false;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Piece {
        Piece { s: s.into(), atom: false, hard_break: false }
    }

    #[test]
    fn wraps_at_width() {
        let spec = WrapSpec { width: 10, first_column: 0, prefix: "> ".into(), eol: "\n".into() };
        assert_eq!(wrap_pieces(&[text("aaaa bbbb cccc")], &spec), "aaaa bbbb\n> cccc");
    }

    #[test]
    fn never_starts_line_with_block_syntax() {
        let spec = WrapSpec { width: 6, first_column: 0, prefix: String::new(), eol: "\n".into() };
        assert_eq!(wrap_pieces(&[text("aaaa - bb")], &spec), "aaaa -\nbb");
    }

    #[test]
    fn split_keeps_runs() {
        assert_eq!(split_spaces("a  b "), vec!["a", "  ", "b", " ", ""]);
    }
}
