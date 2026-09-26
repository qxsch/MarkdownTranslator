//! Splicing translations into the source and structural fingerprints of documents. Port of `document.ts`.

use super::ast::{Kind, Node};
use super::html_attrs::tag_skeleton;
use super::inline::{inline_signature, jsx_skeleton};
use super::parse::parse_markdown;
use super::render::render_segment;
use super::yaml;
use crate::code::parsers::code_fingerprint;
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::{Extraction, ParseOptions, RenderOptions, TMap};

pub fn apply_translations(ex: &Extraction, tm: &TMap, opts: RenderOptions) -> String {
    let mut out = String::with_capacity(ex.source.len() + ex.source.len() / 4);
    let mut pos = 0;
    for r in &ex.replacements {
        out.push_str(&ex.source[pos..r.start]);
        match ex.get(&r.seg) {
            Some(seg) => out.push_str(&render_segment(seg, tm, &ex.store, opts)),
            None => out.push_str(&ex.source[r.start..r.end]),
        }
        pos = r.end;
    }
    out.push_str(&ex.source[pos..]);
    out
}

rx!(HTML_TOKEN, r"<!--[\s\S]*?-->|<[^<>]*>");

pub fn html_skeleton(html: &str) -> String {
    HTML_TOKEN().all(html).into_iter().map(|m| if m.text.starts_with("<!--") { m.text.to_string() } else { tag_skeleton(m.text) }).collect()
}

rx!(INJECTED_ANCHOR, r#"^(?:<a\s+id="[^"]*">|<\/a>)$"#);

/// Heading anchors inserted by `preserve_anchors` (HTML in Markdown, JSX in MDX).
fn is_injected_anchor(c: &Node) -> bool {
    match &c.kind {
        Kind::Html { value } => INJECTED_ANCHOR().test(value),
        Kind::MdxJsxTextElement { name: Some(name), attributes } => name == "a" && c.children.is_empty() && attributes.len() == 1,
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonItem {
    pub sig: String,
    pub start: usize,
    pub end: usize,
}

fn attributes_json(attrs: &[(String, String)]) -> String {
    let parts: Vec<String> = attrs.iter().map(|(k, v)| format!("{}:{}", crate::jsstr::json_string(k), crate::jsstr::json_string(v))).collect();
    format!("{{{}}}", parts.join(","))
}

/// Structural fingerprint of a Markdown document: everything except translatable text.
pub fn skeleton(text: &str, opts: ParseOptions) -> Result<Vec<SkeletonItem>, String> {
    let tree = parse_markdown(text, opts)?;
    let mut out = Vec::new();
    fn push(out: &mut Vec<SkeletonItem>, n: &Node, sig: String) {
        out.push(SkeletonItem { sig, start: n.start, end: n.end });
    }
    fn visit(n: &Node, out: &mut Vec<SkeletonItem>) {
        match &n.kind {
            Kind::Paragraph { .. } => return push(out, n, format!("p\n{}", inline_signature(&n.children, false))),
            Kind::Heading { depth } => {
                let kept: Vec<Node> = n.children.iter().filter(|c| !is_injected_anchor(c)).cloned().collect();
                return push(out, n, format!("h{depth}\n{}", inline_signature(&kept, false)));
            }
            Kind::TableCell => return push(out, n, format!("td\n{}", inline_signature(&n.children, true))),
            Kind::Code { value, lang, meta } => {
                let body = lang.as_deref().filter(|l| !l.is_empty()).and_then(|l| code_fingerprint(l, value)).unwrap_or_else(|| value.clone());
                return push(out, n, format!("code:{}:{}:{}", lang.as_deref().unwrap_or(""), meta.as_deref().unwrap_or(""), body));
            }
            Kind::Html { value } => return push(out, n, format!("html:{}", html_skeleton(value))),
            Kind::Yaml { value } => return push(out, n, format!("yaml:{}", yaml::skeleton(value))),
            Kind::Definition { identifier, url, .. } => return push(out, n, format!("def:{identifier}:{url}")),
            Kind::List { ordered, start } => push(out, n, format!("list:{ordered}:{}", start.map(|s| s.to_string()).unwrap_or_default())),
            Kind::ListItem { checked } => push(out, n, format!("li:{}", checked.map(|c| c.to_string()).unwrap_or_default())),
            Kind::Table { align } => push(out, n, format!("table:{}", align.iter().map(|a| a.unwrap_or("")).collect::<Vec<_>>().join(","))),
            Kind::FootnoteDefinition { identifier, .. } => push(out, n, format!("fndef:{identifier}")),
            Kind::Math { value } => return push(out, n, format!("math:{value}")),
            Kind::ContainerDirective { name, attributes } => push(out, n, format!("dir:{name}:{}", attributes_json(attributes))),
            Kind::LeafDirective { name, attributes } => {
                return push(out, n, format!("leaf:{name}:{}\n{}", attributes_json(attributes), inline_signature(&n.children, false)));
            }
            Kind::MdxjsEsm { value } | Kind::MdxFlowExpression { value } => return push(out, n, format!("{}:{value}", n.type_name())),
            Kind::MdxJsxFlowElement { .. } => push(out, n, format!("jsx:{}", jsx_skeleton(n))),
            Kind::Root => {}
            _ => push(out, n, n.type_name().to_string()),
        }
        for c in &n.children {
            visit(c, out);
        }
    }
    visit(&tree, &mut out);
    Ok(out)
}

/// Index of the first differing skeleton item, or `None` when identical.
pub fn compare_skeletons(a: &[SkeletonItem], b: &[SkeletonItem]) -> Option<usize> {
    let n = a.len().max(b.len());
    (0..n).find(|&i| a.get(i).map(|x| &x.sig) != b.get(i).map(|x| &x.sig))
}
