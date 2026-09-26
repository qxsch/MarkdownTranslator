//! Offset helpers. Port of `lineMap.ts`; widths are UTF-16 code units like the TypeScript implementation.

use crate::jsstr::{line_start, u16len};

/// Maps offsets inside a node's `value` (container prefixes and fence indentation removed) back to offsets in
/// the full source. `None` when a line cannot be matched.
pub struct LineMap {
    deltas: Vec<(usize, isize)>,
}

impl LineMap {
    pub fn build(src: &str, first_source_line_start: usize, value: &str) -> Option<LineMap> {
        let mut deltas = Vec::new();
        let mut src_pos = first_source_line_start;
        let mut v_pos = 0usize;
        for line in value.split('\n') {
            let v_line = line.strip_suffix('\r').unwrap_or(line);
            let src_end = if src_pos <= src.len() { src[src_pos..].find('\n').map(|i| i + src_pos).unwrap_or(src.len()) } else { src.len() };
            let src_start = src_pos.min(src.len());
            let s_line = &src[src_start..src_end];
            let s_line = s_line.strip_suffix('\r').unwrap_or(s_line);
            if !s_line.ends_with(v_line) {
                return None;
            }
            deltas.push((v_pos, (src_start + s_line.len() - v_line.len()) as isize - v_pos as isize));
            v_pos += line.len() + 1;
            src_pos = src_end + 1;
        }
        Some(LineMap { deltas })
    }

    pub fn map(&self, v: usize) -> usize {
        let mut d = self.deltas[0].1;
        for (start, delta) in &self.deltas {
            if *start <= v {
                d = *delta;
            } else {
                break;
            }
        }
        (v as isize + d) as usize
    }
}

pub fn line_start_of(src: &str, offset: usize) -> usize {
    line_start(src, offset)
}

/// Longest line (in UTF-16 code units, without line terminators) covering `[from, to)`.
pub fn max_line_width(src: &str, from: usize, to: usize) -> usize {
    let start = line_start(src, from);
    let end = src[to.min(src.len())..].find('\n').map(|i| i + to).unwrap_or(src.len());
    src[start..end].split('\n').map(|l| u16len(l.strip_suffix('\r').unwrap_or(l))).max().unwrap_or(0)
}

/// Column of `offset` in its line, in UTF-16 code units.
pub fn column_u16(src: &str, offset: usize) -> usize {
    u16len(&src[line_start(src, offset)..offset])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_container_prefixes() {
        let src = "> <div>\n> text\n> </div>\n";
        let value = "<div>\ntext\n</div>";
        let m = LineMap::build(src, 0, value).unwrap();
        assert_eq!(m.map(0), 2);
        assert_eq!(m.map(6), 10);
        assert_eq!(&src[m.map(6)..m.map(10)], "text");
    }

    #[test]
    fn widths() {
        assert_eq!(max_line_width("ab\nabcd\nx", 3, 5), 4);
        assert_eq!(max_line_width("äöü\r\n", 0, 2), 3);
    }
}
