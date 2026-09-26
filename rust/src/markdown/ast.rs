//! Owned Markdown syntax tree (mdast shape) with byte offsets.
//!
//! Converted from markdown-rs's mdast, with fixes that make it match mdast-util-from-markdown (which the
//! TypeScript implementation uses) where markdown-rs differs:
//!
//! * a text node that starts with a character escape starts at the backslash, not after it;
//! * GFM literal autolinks are only recognized where micromark's syntax extension recognizes them (not
//!   while a `[` label start before them is unbalanced; the TypeScript code drops the mdast transform that
//!   would find the rest);
//! * reference identifiers are normalized like micromark (markdown-rs drops a space);
//! * lists, list items and footnote definitions end with their last child (no trailing line ending).

use markdown::mdast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Shortcut,
    Collapsed,
    Full,
}

impl RefKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RefKind::Shortcut => "shortcut",
            RefKind::Collapsed => "collapsed",
            RefKind::Full => "full",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsxAttrValue {
    None,
    Literal(String),
    Expression(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsxAttr {
    /// `{...props}`
    Expression(String),
    Property { name: String, value: JsxAttrValue },
}

#[derive(Clone, Debug)]
pub enum Kind {
    Root,
    Blockquote,
    List { ordered: bool, start: Option<u32> },
    ListItem { checked: Option<bool> },
    Paragraph { directive_label: bool },
    Heading { depth: u8 },
    ThematicBreak,
    Code { value: String, lang: Option<String>, meta: Option<String> },
    Html { value: String },
    Math { value: String },
    Yaml { value: String },
    Toml { value: String },
    Definition { identifier: String, label: Option<String>, url: String, title: Option<String> },
    FootnoteDefinition { identifier: String, label: Option<String> },
    Table { align: Vec<Option<&'static str>> },
    TableRow,
    TableCell,
    Text { value: String },
    Emphasis,
    Strong,
    Delete,
    InlineCode { value: String },
    InlineMath { value: String },
    Break,
    Link { url: String, title: Option<String> },
    LinkReference { identifier: String, label: Option<String>, reference: RefKind },
    Image { url: String, title: Option<String>, alt: String },
    ImageReference { identifier: String, label: Option<String>, reference: RefKind, alt: String },
    FootnoteReference { identifier: String, label: Option<String> },
    ContainerDirective { name: String, attributes: Vec<(String, String)> },
    LeafDirective { name: String, attributes: Vec<(String, String)> },
    MdxjsEsm { value: String },
    MdxFlowExpression { value: String },
    MdxTextExpression { value: String },
    MdxJsxFlowElement { name: Option<String>, attributes: Vec<JsxAttr> },
    MdxJsxTextElement { name: Option<String>, attributes: Vec<JsxAttr> },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(kind: Kind, start: usize, end: usize) -> Node {
        Node { kind, start, end, children: Vec::new() }
    }

    /// The mdast `type` name.
    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            Kind::Root => "root",
            Kind::Blockquote => "blockquote",
            Kind::List { .. } => "list",
            Kind::ListItem { .. } => "listItem",
            Kind::Paragraph { .. } => "paragraph",
            Kind::Heading { .. } => "heading",
            Kind::ThematicBreak => "thematicBreak",
            Kind::Code { .. } => "code",
            Kind::Html { .. } => "html",
            Kind::Math { .. } => "math",
            Kind::Yaml { .. } => "yaml",
            Kind::Toml { .. } => "toml",
            Kind::Definition { .. } => "definition",
            Kind::FootnoteDefinition { .. } => "footnoteDefinition",
            Kind::Table { .. } => "table",
            Kind::TableRow => "tableRow",
            Kind::TableCell => "tableCell",
            Kind::Text { .. } => "text",
            Kind::Emphasis => "emphasis",
            Kind::Strong => "strong",
            Kind::Delete => "delete",
            Kind::InlineCode { .. } => "inlineCode",
            Kind::InlineMath { .. } => "inlineMath",
            Kind::Break => "break",
            Kind::Link { .. } => "link",
            Kind::LinkReference { .. } => "linkReference",
            Kind::Image { .. } => "image",
            Kind::ImageReference { .. } => "imageReference",
            Kind::FootnoteReference { .. } => "footnoteReference",
            Kind::ContainerDirective { .. } => "containerDirective",
            Kind::LeafDirective { .. } => "leafDirective",
            Kind::MdxjsEsm { .. } => "mdxjsEsm",
            Kind::MdxFlowExpression { .. } => "mdxFlowExpression",
            Kind::MdxTextExpression { .. } => "mdxTextExpression",
            Kind::MdxJsxFlowElement { .. } => "mdxJsxFlowElement",
            Kind::MdxJsxTextElement { .. } => "mdxJsxTextElement",
        }
    }

    /// The node's `value` field, for node types that have one.
    pub fn value(&self) -> Option<&str> {
        match &self.kind {
            Kind::Text { value }
            | Kind::InlineCode { value }
            | Kind::InlineMath { value }
            | Kind::Html { value }
            | Kind::Code { value, .. }
            | Kind::Math { value }
            | Kind::Yaml { value }
            | Kind::Toml { value }
            | Kind::MdxjsEsm { value }
            | Kind::MdxFlowExpression { value }
            | Kind::MdxTextExpression { value } => Some(value),
            _ => None,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self.kind, Kind::Text { .. })
    }

    pub fn jsx_name(&self) -> Option<&str> {
        match &self.kind {
            Kind::MdxJsxFlowElement { name, .. } | Kind::MdxJsxTextElement { name, .. } => name.as_deref(),
            _ => None,
        }
    }

    pub fn jsx_attributes(&self) -> &[JsxAttr] {
        match &self.kind {
            Kind::MdxJsxFlowElement { attributes, .. } | Kind::MdxJsxTextElement { attributes, .. } => attributes,
            _ => &[],
        }
    }
}

/// `mdast-util-to-string`: plain text content of a node (values, image alt, children).
pub fn to_string(n: &Node) -> String {
    if let Some(v) = n.value() {
        return v.to_string();
    }
    match &n.kind {
        Kind::Image { alt, .. } | Kind::ImageReference { alt, .. } if !alt.is_empty() => return alt.clone(),
        _ => {}
    }
    n.children.iter().map(to_string).collect()
}

/// `normalizeIdentifier(label).toLowerCase()` as done by micromark and mdast-util-from-markdown.
pub fn normalize_identifier(label: &str) -> String {
    let mut collapsed = String::with_capacity(label.len());
    let mut ws = false;
    for c in label.chars() {
        if matches!(c, '\t' | '\n' | '\r' | ' ') {
            if !ws {
                collapsed.push(' ');
            }
            ws = true;
        } else {
            collapsed.push(c);
            ws = false;
        }
    }
    let trimmed = collapsed.strip_prefix(' ').unwrap_or(&collapsed);
    let trimmed = trimmed.strip_suffix(' ').unwrap_or(trimmed);
    trimmed.to_lowercase().to_uppercase().to_lowercase()
}

fn pos(p: Option<&markdown::unist::Position>) -> (usize, usize) {
    p.map(|p| (p.start.offset, p.end.offset)).unwrap_or((0, 0))
}

fn ref_kind(k: &mdast::ReferenceKind) -> RefKind {
    match k {
        mdast::ReferenceKind::Shortcut => RefKind::Shortcut,
        mdast::ReferenceKind::Collapsed => RefKind::Collapsed,
        mdast::ReferenceKind::Full => RefKind::Full,
    }
}

fn identifier(label: &Option<String>, fallback: &str) -> String {
    match label {
        Some(l) => normalize_identifier(l),
        None => fallback.to_string(),
    }
}

fn jsx_attrs(attrs: &[mdast::AttributeContent]) -> Vec<JsxAttr> {
    attrs
        .iter()
        .map(|a| match a {
            mdast::AttributeContent::Expression(e) => JsxAttr::Expression(e.value.clone()),
            mdast::AttributeContent::Property(p) => JsxAttr::Property {
                name: p.name.clone(),
                value: match &p.value {
                    None => JsxAttrValue::None,
                    Some(mdast::AttributeValue::Literal(s)) => JsxAttrValue::Literal(s.clone()),
                    Some(mdast::AttributeValue::Expression(e)) => JsxAttrValue::Expression(e.value.clone()),
                },
            },
        })
        .collect()
}

/// Converts a markdown-rs tree (positions refer to `src`).
pub fn convert(n: &mdast::Node) -> Node {
    use mdast::Node as M;
    let (start, end) = pos(n.position());
    let kind = match n {
        M::Root(_) => Kind::Root,
        M::Blockquote(_) => Kind::Blockquote,
        M::FootnoteDefinition(x) => Kind::FootnoteDefinition { identifier: identifier(&x.label, &x.identifier), label: x.label.clone() },
        M::MdxJsxFlowElement(x) => Kind::MdxJsxFlowElement { name: x.name.clone(), attributes: jsx_attrs(&x.attributes) },
        M::List(x) => Kind::List { ordered: x.ordered, start: x.start },
        M::MdxjsEsm(x) => Kind::MdxjsEsm { value: x.value.clone() },
        M::Toml(x) => Kind::Toml { value: x.value.clone() },
        M::Yaml(x) => Kind::Yaml { value: x.value.clone() },
        M::Break(_) => Kind::Break,
        M::InlineCode(x) => Kind::InlineCode { value: x.value.clone() },
        M::InlineMath(x) => Kind::InlineMath { value: x.value.clone() },
        M::Delete(_) => Kind::Delete,
        M::Emphasis(_) => Kind::Emphasis,
        M::MdxTextExpression(x) => Kind::MdxTextExpression { value: x.value.clone() },
        M::FootnoteReference(x) => Kind::FootnoteReference { identifier: identifier(&x.label, &x.identifier), label: x.label.clone() },
        M::Html(x) => Kind::Html { value: x.value.clone() },
        M::Image(x) => Kind::Image { url: x.url.clone(), title: x.title.clone(), alt: x.alt.clone() },
        M::ImageReference(x) => Kind::ImageReference {
            identifier: identifier(&x.label, &x.identifier),
            label: x.label.clone(),
            reference: ref_kind(&x.reference_kind),
            alt: x.alt.clone(),
        },
        M::MdxJsxTextElement(x) => Kind::MdxJsxTextElement { name: x.name.clone(), attributes: jsx_attrs(&x.attributes) },
        M::Link(x) => Kind::Link { url: x.url.clone(), title: x.title.clone() },
        M::LinkReference(x) => Kind::LinkReference {
            identifier: identifier(&x.label, &x.identifier),
            label: x.label.clone(),
            reference: ref_kind(&x.reference_kind),
        },
        M::Strong(_) => Kind::Strong,
        M::Text(x) => Kind::Text { value: x.value.clone() },
        M::Code(x) => Kind::Code { value: x.value.clone(), lang: x.lang.clone(), meta: x.meta.clone() },
        M::Math(x) => Kind::Math { value: x.value.clone() },
        M::MdxFlowExpression(x) => Kind::MdxFlowExpression { value: x.value.clone() },
        M::Heading(x) => Kind::Heading { depth: x.depth },
        M::Table(x) => Kind::Table {
            align: x
                .align
                .iter()
                .map(|a| match a {
                    mdast::AlignKind::Left => Some("left"),
                    mdast::AlignKind::Right => Some("right"),
                    mdast::AlignKind::Center => Some("center"),
                    mdast::AlignKind::None => None,
                })
                .collect(),
        },
        M::ThematicBreak(_) => Kind::ThematicBreak,
        M::TableRow(_) => Kind::TableRow,
        M::TableCell(_) => Kind::TableCell,
        M::ListItem(x) => Kind::ListItem { checked: x.checked },
        M::Definition(x) => Kind::Definition {
            identifier: identifier(&x.label, &x.identifier),
            label: x.label.clone(),
            url: x.url.clone(),
            title: x.title.clone(),
        },
        M::Paragraph(_) => Kind::Paragraph { directive_label: false },
    };
    let children = n.children().map(|cs| cs.iter().map(convert).collect()).unwrap_or_default();
    Node { kind, start, end, children }
}

/// Applies the micromark-compatibility fixes described in the module docs.
pub fn fix_tree(node: &mut Node, src: &str) {
    for c in node.children.iter_mut() {
        fix_tree(c, src);
    }
    if matches!(node.kind, Kind::List { .. } | Kind::ListItem { .. } | Kind::FootnoteDefinition { .. }) {
        if let Some(last) = node.children.last() {
            if last.end < node.end {
                node.end = last.end;
            }
        }
    }
    if has_phrasing(node) {
        fix_escape_starts(node, src);
        if node_is_phrasing_root(node) {
            demote_unbalanced_autolinks(node, src);
        }
    }
}

fn has_phrasing(n: &Node) -> bool {
    n.children.iter().any(|c| c.is_text() || is_inline(c))
}

fn is_inline(n: &Node) -> bool {
    matches!(
        n.kind,
        Kind::Text { .. }
            | Kind::Emphasis
            | Kind::Strong
            | Kind::Delete
            | Kind::InlineCode { .. }
            | Kind::InlineMath { .. }
            | Kind::Break
            | Kind::Link { .. }
            | Kind::LinkReference { .. }
            | Kind::Image { .. }
            | Kind::ImageReference { .. }
            | Kind::FootnoteReference { .. }
            | Kind::Html { .. }
            | Kind::MdxTextExpression { .. }
            | Kind::MdxJsxTextElement { .. }
    )
}

/// Paragraph-like containers whose children form one inline content (one micromark text tokenization).
fn node_is_phrasing_root(n: &Node) -> bool {
    matches!(n.kind, Kind::Paragraph { .. } | Kind::Heading { .. } | Kind::TableCell)
}

fn is_escape_at(src: &[u8], backslash: usize) -> bool {
    if src.get(backslash) != Some(&b'\\') {
        return false;
    }
    match src.get(backslash + 1) {
        Some(c) if c.is_ascii_punctuation() => {}
        _ => return false,
    }
    // The backslash must not be escaped itself.
    let mut n = 0;
    while backslash > n && src[backslash - 1 - n] == b'\\' {
        n += 1;
    }
    n % 2 == 0
}

fn fix_escape_starts(parent: &mut Node, src: &str) {
    let bytes = src.as_bytes();
    let mut prev_end: Option<usize> = None;
    for c in parent.children.iter_mut() {
        if c.is_text() && c.start > 0 && is_escape_at(bytes, c.start - 1) && prev_end.map_or(true, |e| e <= c.start - 1) {
            c.start -= 1;
        }
        prev_end = Some(c.end);
    }
}

/// Literal autolink: a link whose source does not start with `[` (resource/reference) or `<` (autolink).
fn is_literal_autolink(n: &Node, src: &[u8]) -> bool {
    matches!(n.kind, Kind::Link { .. }) && !matches!(src.get(n.start), Some(b'[') | Some(b'<'))
}

/// Ranges inside which brackets are not label starts or ends (code, HTML, autolinks, math, expressions).
fn opaque_ranges(n: &Node, src: &[u8], out: &mut Vec<(usize, usize)>) {
    for c in &n.children {
        match c.kind {
            Kind::InlineCode { .. } | Kind::Html { .. } | Kind::InlineMath { .. } | Kind::MdxTextExpression { .. } | Kind::FootnoteReference { .. } => {
                out.push((c.start, c.end))
            }
            Kind::Link { .. } if src.get(c.start) == Some(&b'<') => out.push((c.start, c.end)),
            Kind::Link { .. } | Kind::Image { .. } | Kind::LinkReference { .. } | Kind::ImageReference { .. } => {
                // The destination, title or reference after the label is not inline content.
                let open = if src.get(c.start) == Some(&b'!') { c.start + 1 } else { c.start };
                if src.get(open) == Some(&b'[') {
                    if let Some(close) = label_end(src, open, c.end) {
                        out.push((close + 1, c.end));
                    }
                }
                opaque_ranges(c, src, out);
            }
            _ => opaque_ranges(c, src, out),
        }
    }
}

/// Index of the `]` matching the `[` at `open`, honoring escapes and code spans (like `findLabelEnd`).
pub fn label_end(src: &[u8], open: usize, limit: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = open;
    while i < limit {
        match src[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'`' => {
                let mut run = 0;
                while i + run < limit && src[i + run] == b'`' {
                    run += 1;
                }
                let mut j = i + run;
                let mut found = None;
                while j + run <= src.len() {
                    if src[j..].starts_with(&src[i..i + run]) {
                        found = Some(j);
                        break;
                    }
                    j += 1;
                }
                i = match found {
                    Some(close) if close < limit => close + run,
                    _ => i + run,
                };
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn collect_autolinks(n: &Node, src: &[u8], out: &mut Vec<usize>) {
    for c in &n.children {
        if is_literal_autolink(c, src) {
            out.push(c.start);
        } else {
            collect_autolinks(c, src, out);
        }
    }
}

/// micromark's `previousUnbalanced`: a literal autolink is not recognized while a `[` / `![` label start
/// before it is still open.
fn demote_unbalanced_autolinks(root: &mut Node, src: &str) {
    let bytes = src.as_bytes();
    let mut links = Vec::new();
    collect_autolinks(root, bytes, &mut links);
    if links.is_empty() || root.children.is_empty() {
        return;
    }
    let from = root.children[0].start;
    let to = root.children.last().map(|c| c.end).unwrap_or(from);
    let mut opaque = Vec::new();
    opaque_ranges(root, bytes, &mut opaque);
    opaque.sort();
    let mut literal_ranges: Vec<(usize, usize)> = Vec::new();
    collect_literal_ranges(root, bytes, &mut literal_ranges);
    let mut demote = Vec::new();
    let mut depth = 0usize;
    let mut next_link = 0usize;
    links.sort();
    let mut i = from;
    while i < to {
        while next_link < links.len() && links[next_link] <= i {
            if depth > 0 {
                demote.push(links[next_link]);
            }
            next_link += 1;
        }
        if let Some(&(_, e)) = opaque.iter().find(|(s, e)| *s <= i && i < *e) {
            i = e;
            continue;
        }
        if let Some(&(_, e)) = literal_ranges.iter().find(|(s, e)| *s <= i && i < *e) {
            i = e;
            continue;
        }
        match bytes[i] {
            b'\\' if i + 1 < to && bytes[i + 1].is_ascii_punctuation() => {
                i += 2;
                continue;
            }
            b'[' => depth += 1,
            b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    while next_link < links.len() {
        if depth > 0 {
            demote.push(links[next_link]);
        }
        next_link += 1;
    }
    if !demote.is_empty() {
        demote_links(root, &demote);
    }
}

fn collect_literal_ranges(n: &Node, src: &[u8], out: &mut Vec<(usize, usize)>) {
    for c in &n.children {
        if is_literal_autolink(c, src) {
            out.push((c.start, c.end));
        } else {
            collect_literal_ranges(c, src, out);
        }
    }
}

fn demote_links(parent: &mut Node, starts: &[usize]) {
    let mut changed = false;
    for c in parent.children.iter_mut() {
        if matches!(c.kind, Kind::Link { .. }) && starts.contains(&c.start) {
            let value: String = c.children.iter().map(to_string).collect();
            c.kind = Kind::Text { value };
            c.children.clear();
            changed = true;
        } else {
            demote_links(c, starts);
        }
    }
    if changed {
        merge_adjacent_text(parent);
    }
}

fn merge_adjacent_text(parent: &mut Node) {
    let mut out: Vec<Node> = Vec::with_capacity(parent.children.len());
    for c in parent.children.drain(..) {
        if let (Some(last), Kind::Text { value }) = (out.last_mut(), &c.kind) {
            if let Kind::Text { value: lv } = &mut last.kind {
                if last.end == c.start {
                    lv.push_str(value);
                    last.end = c.end;
                    continue;
                }
            }
        }
        out.push(c);
    }
    parent.children = out;
}

/// Pre-order walk.
pub fn walk<'a>(n: &'a Node, f: &mut impl FnMut(&'a Node)) {
    f(n);
    for c in &n.children {
        walk(c, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers() {
        assert_eq!(normalize_identifier("Contoso  Sync"), "contoso sync");
        assert_eq!(normalize_identifier(" a\n b "), "a b");
    }
}
