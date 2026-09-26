//! Masking of phrasing content (paragraphs, headings, cells) and inline structure signatures. Port of
//! `markdown/inline.ts`.

use super::ast::{label_end, JsxAttr, JsxAttrValue, Kind, Node, RefKind};
use super::context::{ExtractContext, SegmentOptions};
use super::html_attrs::{has_no_translate_marker, is_closing_tag, is_self_closing, tag_name, tag_skeleton, AttrSet, NO_TRANSLATE_ELEMENTS, TRANSLATABLE_JSX_PROPS, VOID_ELEMENTS};
use crate::jsstr;
use crate::mask::masking::{EscapeMode, MaskBuilder};
use crate::regexutil::{has_letter, RegexExt};
use crate::rx;
use crate::types::{PairKind, Render, SegmentKind, TextContext};
use std::collections::HashMap;

/// Index of the `]` matching the `[` at `open`, honoring escapes and code spans.
pub fn find_label_end(src: &str, open: usize, limit: usize) -> Option<usize> {
    label_end(src.as_bytes(), open, limit)
}

rx!(DESTINATION, r"^\]\(\s*(?:<(?:\\.|[^\\<>\n])*>|(?:\\.|[^\s\\()]|\((?:\\.|[^\s\\()])*\))*)\s+");
rx!(TITLE_END, r#"(["')])(\s*\))$"#);

pub struct TitleParts<'a> {
    pub before: &'a str,
    pub raw: &'a str,
    pub after: &'a str,
    pub quote: char,
}

/// Splits `](url "title")` into the part before the title text, the raw title and the rest.
pub fn split_title(close: &str) -> Option<TitleParts<'_>> {
    let end = TITLE_END().exec(close)?;
    let dest = DESTINATION().exec(close)?;
    let q = end.g(1).chars().next()?;
    let opening = if q == ')' { '(' } else { q };
    let start = dest.len();
    let end_index = end.start;
    if close[start..].chars().next() != Some(opening) || start + 1 > end_index {
        return None;
    }
    Some(TitleParts { before: &close[..start + 1], raw: &close[start + 1..end_index], after: &close[end_index..], quote: opening })
}

/// Maps the index of an opening inline HTML tag to the index of its closing sibling.
fn pair_html(nodes: &[Node]) -> HashMap<usize, usize> {
    let mut pairs = HashMap::new();
    let mut stack: Vec<(String, usize)> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        let Kind::Html { value } = &node.kind else { continue };
        let tag = jsstr::trim(value);
        let Some(name) = tag_name(tag) else { continue };
        if is_closing_tag(tag) {
            let Some(idx) = stack.iter().rposition(|(n, _)| *n == name) else { continue };
            pairs.insert(stack[idx].1, i);
            stack.truncate(idx);
        } else if !is_self_closing(tag) && !VOID_ELEMENTS.contains(&name.as_str()) {
            stack.push((name, i));
        }
    }
    pairs
}

fn is_no_translate_tag(tag: &str) -> bool {
    tag_name(tag).is_some_and(|n| NO_TRANSLATE_ELEMENTS.contains(&n.as_str())) || has_no_translate_marker(tag)
}

fn js_name(name: Option<&str>) -> &str {
    // `${node.name}` in the TypeScript code renders a missing (fragment) name as "null".
    name.unwrap_or("null")
}

pub fn walk_phrasing(ctx: &mut ExtractContext, nodes: &[Node], from: usize, to: usize, mb: &mut MaskBuilder, deps: &mut Vec<String>) {
    let src = ctx.source;
    let html_pairs = pair_html(nodes);
    let mut pos = from;
    let mut i = 0;
    while i < nodes.len() {
        let node = &nodes[i];
        let (s, e) = (node.start, node.end);
        if s > pos {
            mb.source(&src[pos..s], EscapeMode::Markdown);
        }
        pos = e;
        match &node.kind {
            Kind::Text { .. } => mb.source(&src[s..e], EscapeMode::Markdown),
            Kind::Emphasis | Kind::Strong | Kind::Delete => {
                if node.children.is_empty() {
                    mb.literal(&src[s..e]);
                } else {
                    let cs = node.children[0].start;
                    let ce = node.children[node.children.len() - 1].end;
                    let kind = match node.kind {
                        Kind::Emphasis => PairKind::Emphasis,
                        Kind::Strong => PairKind::Strong,
                        _ => PairKind::Delete,
                    };
                    let n = mb.open(kind, Render::literal(&src[s..cs]), Render::literal(&src[ce..e]), kind.as_str());
                    walk_phrasing(ctx, &node.children, cs, ce, mb, deps);
                    mb.close(n);
                }
            }
            Kind::InlineCode { .. } => {
                mb.literal(&src[s..e]);
            }
            Kind::Break => {
                let next = if i + 1 < nodes.len() { nodes[i + 1].start } else { to };
                mb.placeholder(Render::literal(&src[s..next]), "line break", true);
                pos = next;
            }
            Kind::Link { .. } | Kind::LinkReference { .. } => link_like(ctx, node, mb, deps),
            Kind::Image { .. } | Kind::ImageReference { .. } => image_like(ctx, node, mb, deps),
            Kind::MdxJsxTextElement { name, .. } => {
                let tag_end = node.children.first().map(|c| c.start).unwrap_or(e);
                let tag = &src[s..tag_end];
                let label = format!("<{}> component", js_name(name.as_deref()));
                if name.as_deref().is_some_and(|n| NO_TRANSLATE_ELEMENTS.contains(&n.to_lowercase().as_str())) || has_no_translate_marker(tag) || node.children.is_empty() {
                    let (render, ids) = if node.children.is_empty() { ctx.tag_with_attributes(tag, &label, AttrSet::Jsx) } else { (Render::literal(&src[s..e]), Vec::new()) };
                    deps.extend(ids);
                    mb.placeholder(render, &src[s..e], false);
                } else {
                    let ce = node.children[node.children.len() - 1].end;
                    let (render, ids) = ctx.tag_with_attributes(tag, &label, AttrSet::Jsx);
                    deps.extend(ids);
                    let n = mb.open(PairKind::Html, render, Render::literal(&src[ce..e]), &label);
                    walk_phrasing(ctx, &node.children, tag_end, ce, mb, deps);
                    mb.close(n);
                }
            }
            Kind::Html { .. } => {
                let tag = &src[s..e];
                if let Some(&close_idx) = html_pairs.get(&i) {
                    let close = &nodes[close_idx];
                    if is_no_translate_tag(tag) {
                        mb.literal(&src[s..close.end]);
                    } else {
                        let (render, ids) = ctx.tag_with_attributes(tag, "inline HTML", AttrSet::Html);
                        deps.extend(ids);
                        let n = mb.open(PairKind::Html, render, Render::literal(&src[close.start..close.end]), tag);
                        walk_phrasing(ctx, &nodes[i + 1..close_idx], e, close.start, mb, deps);
                        mb.close(n);
                    }
                    pos = close.end;
                    i = close_idx + 1;
                    continue;
                }
                let (render, ids) = ctx.tag_with_attributes(tag, "inline HTML", AttrSet::Html);
                deps.extend(ids);
                mb.placeholder(render, tag, false);
            }
            _ => {
                mb.literal(&src[s..e]);
            }
        }
        i += 1;
    }
    if pos < to {
        mb.source(&src[pos..to], EscapeMode::Markdown);
    }
}

fn title_segment(ctx: &mut ExtractContext, close: &str, deps: &mut Vec<String>) -> Render {
    let Some(parts) = split_title(close) else {
        return Render::literal(close);
    };
    let mut mb = ctx.builder();
    mb.source(parts.raw, EscapeMode::Markdown);
    let mut o = SegmentOptions::new(SegmentKind::Title, TextContext::MdTitle, parts.raw, "link/image title (tooltip)");
    o.quote = Some(parts.quote);
    o.embedded = true;
    let (before, after) = (parts.before.to_string(), parts.after.to_string());
    match ctx.segment(mb, o) {
        Some(seg) => {
            deps.push(seg.clone());
            Render::Wrapped { before, seg, after }
        }
        None => Render::literal(close),
    }
}

fn link_like(ctx: &mut ExtractContext, node: &Node, mb: &mut MaskBuilder, deps: &mut Vec<String>) {
    let src = ctx.source;
    let (s, e) = (node.start, node.end);
    if node.children.is_empty() || src.as_bytes().get(s) != Some(&b'[') {
        mb.literal(&src[s..e]);
        return;
    }
    let cs = node.children[0].start;
    let ce = node.children[node.children.len() - 1].end;
    let raw_close = &src[ce..e];
    let mut reference = None;
    let close = match &node.kind {
        Kind::LinkReference { reference: kind, .. } if *kind != RefKind::Full => {
            // Translating the text of [label] / [label][] would break the reference; keep the label explicitly.
            let label = &src[cs..ce];
            reference = Some((label.to_string(), raw_close.to_string()));
            Render::literal(format!("][{label}]"))
        }
        Kind::Link { title: Some(_), .. } => title_segment(ctx, raw_close, deps),
        _ => Render::literal(raw_close),
    };
    let hint = match &node.kind {
        Kind::Link { url, .. } => format!("link to {url}"),
        Kind::LinkReference { label, .. } => format!("link [{}]", label.as_deref().unwrap_or("")),
        _ => String::new(),
    };
    let n = mb.open(PairKind::Link, Render::literal(&src[s..cs]), close, &hint);
    if let Some((label, close_original)) = reference {
        let pair = mb.pairs.get_mut(&n).unwrap();
        pair.ref_label = Some(label);
        pair.close_original = Some(close_original);
    }
    walk_phrasing(ctx, &node.children, cs, ce, mb, deps);
    mb.close(n);
}

fn image_like(ctx: &mut ExtractContext, node: &Node, mb: &mut MaskBuilder, deps: &mut Vec<String>) {
    let src = ctx.source;
    let (s, e) = (node.start, node.end);
    let alt_end = find_label_end(src, s + 1, e);
    let alt_raw = alt_end.map(|a| &src[s + 2..a]).unwrap_or("");
    let Some(alt_end) = alt_end.filter(|_| has_letter(alt_raw)) else {
        mb.literal(&src[s..e]);
        return;
    };
    let raw_close = &src[alt_end..e];
    let short_ref = matches!(&node.kind, Kind::ImageReference { reference, .. } if *reference != RefKind::Full);
    let close = if short_ref {
        Render::literal(format!("][{alt_raw}]"))
    } else if matches!(&node.kind, Kind::Image { title: Some(_), .. }) {
        title_segment(ctx, raw_close, deps)
    } else {
        Render::literal(raw_close)
    };
    let hint = match &node.kind {
        Kind::Image { url, .. } => format!("image {url}"),
        _ => "image".to_string(),
    };
    let n = mb.open(PairKind::Image, Render::literal(&src[s..s + 2]), close, &hint);
    if short_ref {
        let pair = mb.pairs.get_mut(&n).unwrap();
        pair.ref_label = Some(alt_raw.to_string());
        pair.close_original = Some(raw_close.to_string());
    }
    mb.source(alt_raw, EscapeMode::Markdown);
    mb.close(n);
}

/// JSX element name and attributes, with translatable attribute values blanked.
pub fn jsx_skeleton(node: &Node) -> String {
    let attrs: Vec<String> = node
        .jsx_attributes()
        .iter()
        .map(|a| match a {
            JsxAttr::Expression(v) => format!("{{{v}}}"),
            JsxAttr::Property { name, value } => {
                let v = match value {
                    JsxAttrValue::Literal(s) => {
                        if TRANSLATABLE_JSX_PROPS.contains(&name.to_lowercase().as_str()) {
                            "…".to_string()
                        } else {
                            s.clone()
                        }
                    }
                    JsxAttrValue::None => String::new(),
                    JsxAttrValue::Expression(x) => format!("{{{x}}}"),
                };
                format!("{name}={v}")
            }
        })
        .collect();
    format!("{} {}", node.jsx_name().unwrap_or(""), attrs.join(" "))
}

rx!(ESCAPED_PIPE, r"\\\|");

/// Order-independent signature of the inline structure (everything except text).
pub fn inline_signature(nodes: &[Node], cell: bool) -> String {
    let mut items: Vec<String> = Vec::new();
    fn norm(v: &str, cell: bool) -> String {
        if cell {
            ESCAPED_PIPE().replace_all_str(v, "|")
        } else {
            v.to_string()
        }
    }
    fn visit(n: &Node, cell: bool, items: &mut Vec<String>) {
        match &n.kind {
            Kind::Text { .. } => return,
            Kind::InlineCode { value } => {
                items.push(format!("code:{}", ESCAPED_PIPE().replace_all_str(&norm(value, cell), "|")));
                return;
            }
            Kind::Link { url, .. } => items.push(format!("link:{url}")),
            Kind::Image { url, .. } => {
                items.push(format!("image:{url}"));
                return;
            }
            Kind::LinkReference { identifier, .. } => items.push(format!("linkref:{identifier}")),
            Kind::ImageReference { identifier, .. } => {
                items.push(format!("imageref:{identifier}"));
                return;
            }
            Kind::FootnoteReference { identifier, .. } => {
                items.push(format!("fn:{identifier}"));
                return;
            }
            Kind::Html { value } => {
                items.push(format!("html:{}", tag_skeleton(value)));
                return;
            }
            Kind::InlineMath { value } => {
                items.push(format!("math:{value}"));
                return;
            }
            Kind::MdxTextExpression { value } => {
                items.push(format!("expr:{value}"));
                return;
            }
            Kind::MdxJsxTextElement { .. } => items.push(format!("jsx:{}", jsx_skeleton(n))),
            _ => items.push(n.type_name().to_string()),
        }
        for c in &n.children {
            visit(c, cell, items);
        }
    }
    for n in nodes {
        visit(n, cell, &mut items);
    }
    items.sort_by(|a, b| jsstr::cmp_u16(a, b));
    items.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        let t = split_title("](https://a.b \"Home page\")").unwrap();
        assert_eq!((t.before, t.raw, t.after, t.quote), ("](https://a.b \"", "Home page", "\")", '"'));
        let p = split_title("](x (paren title))").unwrap();
        assert_eq!((p.raw, p.quote), ("paren title", '('));
        assert!(split_title("](x)").is_none());
    }
}
