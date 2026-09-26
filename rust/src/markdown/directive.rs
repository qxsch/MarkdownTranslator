//! Block directives (`:::name[label]{attrs}` containers and `::name[label]{attrs}` leaves), the subset of
//! micromark-extension-directive the TypeScript implementation enables.
//!
//! markdown-rs has no directive support, so they are parsed in two passes:
//!
//! 1. The document is parsed without directives. Lines outside raw blocks (code, HTML, math, front matter,
//!    ESM) whose content is valid directive fence syntax are matched into open/close pairs, following
//!    micromark's rules: the outermost open container checks each line first, so a closing fence closes
//!    the outermost open container whose fence is not longer than its own.
//! 2. Each fence line is replaced by a thematic break of the same byte length (`*****`), which interrupts
//!    paragraphs and lists exactly where a directive fence does, and the document is parsed again. The
//!    breaks are then turned back into directive nodes, grouping the siblings between an open fence and
//!    its closing fence. Labels are parsed as phrasing content on their own.

use super::ast::{Kind, Node};
use crate::types::ParseOptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceKind {
    Leaf,
    Open,
    Close,
}

#[derive(Clone, Debug)]
pub struct Fence {
    pub kind: FenceKind,
    /// Start of the line; markdown-rs starts indented thematic breaks here, not at the first `*`.
    pub line_start: usize,
    /// Byte offset of the first colon.
    pub start: usize,
    /// End of the line content (before the line ending).
    pub end: usize,
    pub colons: usize,
    pub name: String,
    /// Absolute byte range of the label content (inside the brackets) and of the whole `[...]`.
    pub label: Option<(usize, usize)>,
    pub label_outer: Option<(usize, usize)>,
    pub attributes: Vec<(String, String)>,
    /// Index of the matching fence (open <-> close).
    pub pair: Option<usize>,
}

/// Directive fence syntax on one line (without container prefix).
#[derive(Clone, Debug)]
struct Syntax {
    kind: FenceKind,
    colons: usize,
    name: String,
    label: Option<(usize, usize)>,
    label_outer: Option<(usize, usize)>,
    attributes: Vec<(String, String)>,
}

fn is_unicode_punctuation(c: char) -> bool {
    // micromark: /\p{P}|\p{S}/u
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"^[\p{P}\p{S}]$").unwrap());
    let mut buf = [0u8; 4];
    re.is_match(c.encode_utf8(&mut buf))
}

fn is_unicode_whitespace(c: char) -> bool {
    crate::jsstr::is_js_space(c)
}

/// Parses `line` (the content after container prefixes) as a directive fence.
fn scan(line: &str) -> Option<Syntax> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let at = |i: usize| chars.get(i).map(|x| x.1);
    let off = |i: usize| chars.get(i).map(|x| x.0).unwrap_or(line.len());
    let mut i = 0;
    while at(i) == Some(':') {
        i += 1;
    }
    let colons = i;
    if colons < 2 {
        return None;
    }
    let rest_blank = |from: usize| chars[from.min(chars.len())..].iter().all(|(_, c)| *c == ' ' || *c == '\t');
    if colons >= 3 && rest_blank(i) {
        return Some(Syntax { kind: FenceKind::Close, colons, name: String::new(), label: None, label_outer: None, attributes: Vec::new() });
    }
    // Name: not punctuation or whitespace at the start; `-` and `_` allowed inside but not at the end.
    let first = at(i)?;
    if is_unicode_punctuation(first) || is_unicode_whitespace(first) {
        return None;
    }
    let name_start = i;
    i += 1;
    while let Some(c) = at(i) {
        if is_unicode_whitespace(c) || (is_unicode_punctuation(c) && c != '-' && c != '_') {
            break;
        }
        i += 1;
    }
    let name: String = chars[name_start..i].iter().map(|x| x.1).collect();
    if name.ends_with('-') || name.ends_with('_') {
        return None;
    }
    let mut label = None;
    let mut label_outer = None;
    if at(i) == Some('[') {
        let open = i;
        i += 1;
        let content_start = i;
        let mut balance = 0usize;
        loop {
            match at(i) {
                None => return None,
                Some('[') => {
                    balance += 1;
                    if balance > 32 {
                        return None;
                    }
                }
                Some(']') => {
                    if balance == 0 {
                        break;
                    }
                    balance -= 1;
                }
                Some('\\') if matches!(at(i + 1), Some('[') | Some('\\') | Some(']')) => i += 1,
                _ => {}
            }
            i += 1;
        }
        label = Some((off(content_start), off(i)));
        label_outer = Some((off(open), off(i + 1)));
        i += 1;
    }
    let mut attributes = Vec::new();
    if at(i) == Some('{') {
        let (attrs, next) = scan_attributes(&chars, i + 1)?;
        attributes = attrs;
        i = next;
    }
    if !rest_blank(i) {
        return None;
    }
    let kind = if colons == 2 { FenceKind::Leaf } else { FenceKind::Open };
    Some(Syntax { kind, colons, name, label, label_outer, attributes })
}

/// micromark's `factoryAttributes` with `disallowEol`; returns the cleaned attributes and the index after `}`.
fn scan_attributes(chars: &[(usize, char)], mut i: usize) -> Option<(Vec<(String, String)>, usize)> {
    let at = |i: usize| chars.get(i).map(|x| x.1);
    let mut list: Vec<(String, String)> = Vec::new();
    let forbidden_shortcut = |c: char| matches!(c, '"' | '\'' | '<' | '=' | '>' | '`');
    loop {
        let c = at(i)?;
        match c {
            '#' | '.' => {
                let key = if c == '#' { "id" } else { "class" };
                i += 1;
                let v0 = at(i)?;
                if forbidden_shortcut(v0) || matches!(v0, '#' | '.' | '}' | ' ' | '\t') {
                    return None;
                }
                let start = i;
                while let Some(c) = at(i) {
                    if matches!(c, '#' | '.' | '}' | ' ' | '\t') {
                        break;
                    }
                    if forbidden_shortcut(c) {
                        return None;
                    }
                    i += 1;
                }
                list.push((key.to_string(), chars[start..i].iter().map(|x| x.1).collect()));
            }
            ' ' | '\t' => i += 1,
            '}' => return Some((clean(list), i + 1)),
            c if is_unicode_whitespace(c) || (is_unicode_punctuation(c) && c != '-' && c != '_') => return None,
            _ => {
                let start = i;
                i += 1;
                while let Some(c) = at(i) {
                    if is_unicode_whitespace(c) || (is_unicode_punctuation(c) && !matches!(c, '-' | '.' | ':' | '_')) {
                        break;
                    }
                    i += 1;
                }
                let name: String = chars[start..i].iter().map(|x| x.1).collect();
                while matches!(at(i), Some(' ') | Some('\t')) {
                    i += 1;
                }
                if at(i) != Some('=') {
                    list.push((name, String::new()));
                    continue;
                }
                i += 1;
                while matches!(at(i), Some(' ') | Some('\t')) {
                    i += 1;
                }
                let v = at(i)?;
                if matches!(v, '<' | '=' | '>' | '`' | '}') {
                    return None;
                }
                if v == '"' || v == '\'' {
                    let start = i + 1;
                    i += 1;
                    while at(i)? != v {
                        i += 1;
                    }
                    list.push((name, chars[start..i].iter().map(|x| x.1).collect()));
                    i += 1;
                    match at(i)? {
                        '}' | ' ' | '\t' => {}
                        _ => return None,
                    }
                } else {
                    let start = i;
                    while let Some(c) = at(i) {
                        if c == '}' || c == ' ' || c == '\t' {
                            break;
                        }
                        if matches!(c, '"' | '\'' | '<' | '=' | '>' | '`') {
                            return None;
                        }
                        i += 1;
                    }
                    at(i)?;
                    list.push((name, chars[start..i].iter().map(|x| x.1).collect()));
                }
            }
        }
    }
}

/// mdast-util-directive: classes are joined, other keys keep their first position with the last value.
fn clean(list: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, v) in list {
        match out.iter_mut().find(|(ok, _)| *ok == k) {
            Some(existing) if k == "class" => {
                existing.1.push(' ');
                existing.1.push_str(&v);
            }
            Some(existing) => existing.1 = v,
            None => out.push((k, v)),
        }
    }
    out
}

/// Byte ranges of raw blocks in the first-pass tree, where `:::` lines are not directive fences.
fn raw_ranges(n: &Node, parent_is_flow: bool, out: &mut Vec<(usize, usize)>) {
    for c in &n.children {
        let flow = matches!(
            n.kind,
            Kind::Root | Kind::Blockquote | Kind::ListItem { .. } | Kind::FootnoteDefinition { .. } | Kind::MdxJsxFlowElement { .. } | Kind::ContainerDirective { .. }
        );
        match &c.kind {
            Kind::Code { .. } | Kind::Math { .. } | Kind::Yaml { .. } | Kind::Toml { .. } | Kind::MdxjsEsm { .. } | Kind::MdxFlowExpression { .. } => {
                out.push((c.start, c.end))
            }
            Kind::Html { .. } if flow => out.push((c.start, c.end)),
            _ => raw_ranges(c, flow, out),
        }
    }
    let _ = parent_is_flow;
}

fn list_item_content_columns(n: &Node, src: &str, out: &mut Vec<(usize, usize, usize)>) {
    for c in &n.children {
        if let Kind::ListItem { .. } = c.kind {
            if let Some(first) = c.children.first() {
                let col = first.start - crate::jsstr::line_start(src, first.start);
                out.push((c.start, c.end, col));
            }
        }
        list_item_content_columns(c, src, out);
    }
}

struct LinePrefix {
    depth: usize,
    /// Byte offset where the directive syntax would start.
    content: usize,
    /// Indentation of the content relative to its container (columns).
    relative_indent: usize,
}

fn line_prefix(src: &str, line_start: usize, line_end: usize, items: &[(usize, usize, usize)]) -> LinePrefix {
    let b = src.as_bytes();
    let mut i = line_start;
    let mut depth = 0;
    loop {
        let mut j = i;
        while j < line_end && j - i < 3 && b[j] == b' ' {
            j += 1;
        }
        if j < line_end && b[j] == b'>' {
            depth += 1;
            i = j + 1;
            if i < line_end && (b[i] == b' ' || b[i] == b'\t') {
                i += 1;
            }
            continue;
        }
        break;
    }
    let after_quote = i;
    let mut marker = false;
    {
        let mut j = i;
        while j < line_end && (b[j] == b' ' || b[j] == b'\t') {
            j += 1;
        }
        let m = j;
        if j < line_end && matches!(b[j], b'-' | b'+' | b'*') {
            j += 1;
        } else {
            let d = j;
            while j < line_end && b[j].is_ascii_digit() && j - d < 9 {
                j += 1;
            }
            if j > d && j < line_end && (b[j] == b'.' || b[j] == b')') {
                j += 1;
            } else {
                j = m;
            }
        }
        if j > m && j < line_end && (b[j] == b' ' || b[j] == b'\t') {
            i = j;
            marker = true;
        }
    }
    let ws_start = i;
    while i < line_end && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    let width = |from: usize, to: usize| b[from..to].iter().map(|c| if *c == b'\t' { 4 } else { 1 }).sum::<usize>();
    let relative_indent = if marker {
        width(ws_start, i).saturating_sub(1)
    } else {
        let col = width(line_start, i);
        let container = items
            .iter()
            .filter(|(s, e, _)| *s < line_start && line_start <= *e)
            .map(|(_, _, c)| *c)
            .max()
            .unwrap_or_else(|| width(line_start, after_quote));
        col.saturating_sub(container)
    };
    LinePrefix { depth, content: i, relative_indent }
}

/// Finds directive fences using the first-pass tree. `excluded` holds fence starts that turned out not to
/// be block boundaries in the second pass.
pub fn plan(src: &str, tree: &Node, excluded: &[usize]) -> Vec<Fence> {
    let mut raw = Vec::new();
    raw_ranges(tree, true, &mut raw);
    let mut items = Vec::new();
    list_item_content_columns(tree, src, &mut items);
    let mut fences: Vec<Fence> = Vec::new();
    // Open containers per blockquote depth: (fence index, colons).
    let mut stacks: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut line_start = 0;
    while line_start <= src.len() {
        let nl = src[line_start..].find('\n').map(|i| i + line_start);
        let mut line_end = nl.unwrap_or(src.len());
        if line_end > line_start && src.as_bytes()[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        let next = nl.map(|i| i + 1);
        let in_raw = raw.iter().any(|(s, e)| *s < line_end.max(line_start + 1) && line_start < *e);
        if !in_raw {
            let p = line_prefix(src, line_start, line_end, &items);
            let blank = src[p.content..line_end].trim().is_empty();
            if blank {
                // A blank line ends deeper blockquotes, and the directives open inside them.
                for s in stacks.iter_mut().skip(p.depth + 1) {
                    s.clear();
                }
            } else if p.relative_indent < 4 && !excluded.contains(&p.content) {
                if let Some(syn) = scan(&src[p.content..line_end]) {
                    for s in stacks.iter_mut().skip(p.depth + 1) {
                        s.clear();
                    }
                    while stacks.len() <= p.depth {
                        stacks.push(Vec::new());
                    }
                    let stack = &mut stacks[p.depth];
                    let shift = |r: (usize, usize)| (r.0 + p.content, r.1 + p.content);
                    let fence = Fence {
                        kind: syn.kind,
                        line_start,
                        start: p.content,
                        end: line_end,
                        colons: syn.colons,
                        name: syn.name,
                        label: syn.label.map(shift),
                        label_outer: syn.label_outer.map(shift),
                        attributes: syn.attributes,
                        pair: None,
                    };
                    match fence.kind {
                        FenceKind::Leaf => fences.push(fence),
                        FenceKind::Open => {
                            fences.push(fence);
                            stack.push((fences.len() - 1, syn.colons));
                        }
                        FenceKind::Close => {
                            if let Some(pos) = stack.iter().position(|(_, c)| *c <= syn.colons) {
                                let open = stack[pos].0;
                                stack.truncate(pos);
                                fences.push(fence);
                                let close = fences.len() - 1;
                                fences[open].pair = Some(close);
                                fences[close].pair = Some(open);
                            }
                        }
                    }
                }
            }
        }
        match next {
            Some(n) => line_start = n,
            None => break,
        }
    }
    fences
}

/// The source with every fence line replaced by a same-length thematic break.
pub fn mask(src: &str, fences: &[Fence]) -> String {
    let mut bytes = src.as_bytes().to_vec();
    for f in fences {
        for b in &mut bytes[f.start..f.end] {
            *b = b'*';
        }
    }
    String::from_utf8(bytes).expect("fence lines are replaced as whole lines")
}

/// Fence starts that are thematic breaks in the second-pass tree.
pub fn break_starts(tree: &Node) -> Vec<usize> {
    let mut out = Vec::new();
    super::ast::walk(tree, &mut |n| {
        if matches!(n.kind, Kind::ThematicBreak) {
            out.push(n.start);
        }
    });
    out
}

impl Fence {
    /// Is `offset` the start of this fence's thematic break in the second pass?
    pub fn is_break_start(&self, offset: usize) -> bool {
        self.line_start <= offset && offset <= self.start
    }
}

pub struct Restructure<'a> {
    pub src: &'a str,
    pub fences: &'a [Fence],
    pub opts: ParseOptions,
    /// Link and footnote definitions, appended when parsing labels on their own.
    pub definitions: String,
}

impl Restructure<'_> {
    fn fence_at(&self, n: &Node) -> Option<usize> {
        if !matches!(n.kind, Kind::ThematicBreak) {
            return None;
        }
        self.fences.iter().position(|f| f.is_break_start(n.start))
    }

    pub fn run(&self, root: &mut Node) {
        let children = std::mem::take(&mut root.children);
        root.children = self.list(children);
    }

    fn list(&self, children: Vec<Node>) -> Vec<Node> {
        let mut out = Vec::with_capacity(children.len());
        let mut rest = std::collections::VecDeque::from(children);
        while let Some(mut n) = rest.pop_front() {
            let Some(fi) = self.fence_at(&n) else {
                if is_flow_container(&n) {
                    let kids = std::mem::take(&mut n.children);
                    n.children = self.list(kids);
                }
                out.push(n);
                continue;
            };
            let f = &self.fences[fi];
            match f.kind {
                FenceKind::Leaf => {
                    let mut leaf = Node::new(Kind::LeafDirective { name: f.name.clone(), attributes: f.attributes.clone() }, f.start, f.end);
                    if let Some(label) = f.label {
                        leaf.children = self.label(label);
                    }
                    out.push(leaf);
                }
                FenceKind::Open => {
                    let close = f.pair.map(|p| &self.fences[p]);
                    let mut inner = Vec::new();
                    let mut end = f.end;
                    let mut closed = false;
                    while let Some(m) = rest.pop_front() {
                        if close.is_some_and(|c| c.is_break_start(m.start)) && matches!(m.kind, Kind::ThematicBreak) {
                            end = close.unwrap().end;
                            closed = true;
                            break;
                        }
                        end = m.end;
                        inner.push(m);
                    }
                    if !closed {
                        // An unclosed container runs to the end of its parent, through the last line ending.
                        let b = self.src.as_bytes();
                        if b.get(end) == Some(&b'\r') {
                            end += 1;
                        }
                        if b.get(end) == Some(&b'\n') {
                            end += 1;
                        }
                    }
                    let mut node = Node::new(Kind::ContainerDirective { name: f.name.clone(), attributes: f.attributes.clone() }, f.start, end);
                    if let (Some(label), Some(outer)) = (f.label, f.label_outer) {
                        let mut p = Node::new(Kind::Paragraph { directive_label: true }, outer.0, outer.1);
                        p.children = self.label(label);
                        node.children.push(p);
                    }
                    node.children.extend(self.list(inner));
                    out.push(node);
                }
                FenceKind::Close => {
                    // A closing fence whose opening ended up in another container is plain text.
                    let mut p = Node::new(Kind::Paragraph { directive_label: false }, f.start, f.end);
                    p.children.push(Node::new(Kind::Text { value: self.src[f.start..f.end].trim_end().to_string() }, f.start, f.end));
                    out.push(p);
                }
            }
        }
        out
    }

    /// Parses a label as phrasing content; positions are mapped back into the document.
    fn label(&self, (from, to): (usize, usize)) -> Vec<Node> {
        let text = &self.src[from..to];
        if text.is_empty() {
            return Vec::new();
        }
        let sub = format!("a {text}\n\n{}", self.definitions);
        let Ok(tree) = super::parse::parse_raw(&sub, self.opts, false) else {
            return vec![Node::new(Kind::Text { value: text.to_string() }, from, to)];
        };
        let Some(first) = tree.children.into_iter().next().filter(|p| matches!(p.kind, Kind::Paragraph { .. })) else {
            return vec![Node::new(Kind::Text { value: text.to_string() }, from, to)];
        };
        let mut kids = first.children;
        for k in kids.iter_mut() {
            shift(k, from, 2);
        }
        if let Some(k) = kids.first_mut() {
            if let Kind::Text { value } = &mut k.kind {
                if let Some(rest) = value.strip_prefix("a ") {
                    *value = rest.to_string();
                }
                k.start = from;
                if value.is_empty() {
                    kids.remove(0);
                }
            }
        }
        kids
    }
}

fn is_flow_container(n: &Node) -> bool {
    matches!(
        n.kind,
        Kind::Root | Kind::Blockquote | Kind::List { .. } | Kind::ListItem { .. } | Kind::FootnoteDefinition { .. } | Kind::MdxJsxFlowElement { .. }
    )
}

/// Maps offsets of a sub-parse (`prefix` bytes before the label) onto the document.
fn shift(n: &mut Node, from: usize, prefix: usize) {
    n.start = (n.start + from).saturating_sub(prefix).max(from);
    n.end = (n.end + from).saturating_sub(prefix).max(from);
    for c in n.children.iter_mut() {
        shift(c, from, prefix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(line: &str) -> Option<FenceKind> {
        scan(line).map(|s| s.kind)
    }

    #[test]
    fn fence_syntax() {
        assert_eq!(kind(":::note"), Some(FenceKind::Open));
        assert_eq!(kind(":::note[Title]{.a #b}  "), Some(FenceKind::Open));
        assert_eq!(kind(":::"), Some(FenceKind::Close));
        assert_eq!(kind("::::  "), Some(FenceKind::Close));
        assert_eq!(kind("::youtube[Watch]{#id}"), Some(FenceKind::Leaf));
        assert_eq!(kind(":::note Title"), None);
        assert_eq!(kind(":::image type=\"content\":::"), None);
        assert_eq!(kind(":::note-"), None);
        assert_eq!(kind("::"), None);
        assert_eq!(kind(":x"), None);
    }

    #[test]
    fn attributes() {
        let s = scan(":::a{.x .y #i k=v q=\"a b\" flag}").unwrap();
        assert_eq!(
            s.attributes,
            vec![
                ("class".to_string(), "x y".to_string()),
                ("id".to_string(), "i".to_string()),
                ("k".to_string(), "v".to_string()),
                ("q".to_string(), "a b".to_string()),
                ("flag".to_string(), String::new()),
            ]
        );
    }
}
