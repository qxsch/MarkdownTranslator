//! Splits a Markdown document into translatable segments. Port of `markdown/extract.ts`.

use super::ast::{to_string, Kind, Node};
use super::context::{ExtractContext, SegmentOptions};
use super::frontmatter::process_frontmatter;
use super::html::process_html_block;
use super::html_attrs::AttrSet;
use super::inline::{inline_signature, walk_phrasing};
use super::line_map::{column_u16, max_line_width};
use super::parse::parse_markdown;
use crate::code::comments::process_code_block;
use crate::jsstr;
use crate::mask::masking::EscapeMode;
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::{ExtractOptions, Extraction, Formality, SegmentKind, TextContext, WrapSpec};

fn clip(s: &str, n: usize) -> String {
    let one = jsstr::trim(&jsstr::collapse_ws(s)).to_string();
    if jsstr::u16len(&one) > n {
        format!("{}\u{2026}", jsstr::prefix_u16(&one, n - 1))
    } else {
        one
    }
}

/// End of a JSX opening tag starting at `start` (quotes and {expressions} may contain `>`).
fn jsx_tag_end(src: &str, start: usize, limit: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    for i in start..limit.min(b.len()) {
        let c = b[i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
        } else if c == b'>' && depth == 0 {
            return Some(i + 1);
        }
    }
    None
}

rx!(DEFINITION_TITLE, r#"^(\[(?:\\.|[^\]\\])*\]:\s*(?:<[^>\n]*>|\S+)\s+)(["'(])([\s\S]*)["')][ \t]*$"#);

struct Extractor<'a, 'c> {
    ctx: &'c mut ExtractContext<'a>,
    definitions: Vec<String>,
    frontmatter_formality: Option<Formality>,
    headings: Vec<Option<String>>,
}

impl Extractor<'_, '_> {
    /// Where a segment sits: section path plus local context (table column, list lead-in, component).
    fn set_structure(&mut self, local: &[String], extra: Option<String>) {
        let section: Vec<&str> = self.headings.iter().filter_map(|h| h.as_deref()).filter(|h| !h.is_empty()).collect();
        let section = section.join(" > ");
        let mut parts: Vec<String> = Vec::new();
        if !section.is_empty() {
            parts.push(format!("section \"{}\"", clip(&section, 160)));
        }
        parts.extend(local.iter().filter(|s| !s.is_empty()).cloned());
        if let Some(e) = extra.filter(|e| !e.is_empty()) {
            parts.push(e);
        }
        self.ctx.structure = if parts.is_empty() { None } else { Some(parts.join("; ")) };
    }

    fn prose(&mut self, children: &[Node], kind: SegmentKind, note: &str) {
        if children.is_empty() {
            return;
        }
        let src = self.ctx.source;
        let from = children[0].start;
        let to = children[children.len() - 1].end;
        let mut mb = self.ctx.builder();
        let mut deps = Vec::new();
        walk_phrasing(self.ctx, children, from, to, &mut mb, &mut deps);
        let wrap = match (&mb.soft_break, kind) {
            (Some(sb), SegmentKind::Paragraph) => Some(WrapSpec {
                width: max_line_width(src, from, to).max(40),
                first_column: column_u16(src, from),
                prefix: sb.prefix.clone(),
                eol: sb.eol.clone(),
            }),
            _ => None,
        };
        let cell = kind == SegmentKind::Cell;
        let mut o = SegmentOptions::new(kind, if cell { TextContext::Cell } else { TextContext::Markdown }, &src[from..to], note);
        o.wrap = wrap;
        o.dependents = deps;
        o.force = true;
        o.inline_signature = Some(inline_signature(children, cell));
        if let Some(seg) = self.ctx.segment(mb, o) {
            self.ctx.replace(from, to, seg);
        }
    }

    fn children(&mut self, parent: &Node, where_: &str, local: &[String]) -> Result<(), String> {
        for (i, c) in parent.children.iter().enumerate() {
            let prev = if i > 0 { parent.children.get(i - 1) } else { None };
            if matches!(c.kind, Kind::List { .. }) && prev.is_some_and(|p| matches!(p.kind, Kind::Paragraph { .. })) {
                let mut l = local.to_vec();
                l.push(format!("list introduced by \"{}\"", clip(&to_string(prev.unwrap()), 80)));
                self.visit(c, where_, &l)?;
            } else {
                self.visit(c, where_, local)?;
            }
        }
        Ok(())
    }

    fn with(local: &[String], extra: String) -> Vec<String> {
        let mut l = local.to_vec();
        l.push(extra);
        l
    }

    fn visit(&mut self, node: &Node, where_: &str, local: &[String]) -> Result<(), String> {
        let src = self.ctx.source;
        match &node.kind {
            Kind::Root => self.children(node, "paragraph", local)?,
            Kind::Blockquote => self.children(node, "paragraph in a blockquote", local)?,
            Kind::List { .. } => self.children(node, "list item", local)?,
            Kind::ListItem { .. } => self.children(node, where_, local)?,
            Kind::FootnoteDefinition { identifier, label } => {
                self.definitions.push(format!("[^{}]: x", label.as_deref().unwrap_or(identifier)));
                self.children(node, "footnote", &Self::with(local, "footnote".into()))?;
            }
            Kind::ContainerDirective { name, .. } => {
                self.children(node, "paragraph", &Self::with(local, format!("inside \"{name}\" admonition/directive")))?;
            }
            Kind::LeafDirective { name, .. } => {
                let label = format!("\"{name}\" directive label");
                self.set_structure(local, Some(label.clone()));
                self.prose(&node.children, SegmentKind::Label, &label);
            }
            Kind::MdxJsxFlowElement { name, .. } => {
                let display = name.as_deref().unwrap_or("null");
                if let Some(tag_end) = jsx_tag_end(src, node.start, node.end) {
                    self.set_structure(local, None);
                    let tag = &src[node.start..tag_end];
                    let (render, ids) = self.ctx.tag_with_attributes(tag, &format!("<{display}> component"), AttrSet::Jsx);
                    if !ids.is_empty() {
                        let mut mb = self.ctx.builder();
                        mb.placeholder(render, tag, false);
                        let mut o = SegmentOptions::new(SegmentKind::Html, TextContext::Html, tag, format!("<{display}> component attributes"));
                        o.dependents = ids;
                        o.force = true;
                        if let Some(seg) = self.ctx.segment(mb, o) {
                            self.ctx.replace(node.start, tag_end, seg);
                        }
                    }
                }
                self.children(node, "paragraph", &Self::with(local, format!("inside <{display}> component")))?;
            }
            Kind::Table { .. } => {
                let header: Vec<String> = node.children.first().map(|r| r.children.iter().map(|c| clip(&to_string(c), 40)).collect()).unwrap_or_default();
                for (r, row) in node.children.iter().enumerate() {
                    let row_label = row.children.first().map(|c| clip(&to_string(c), 40)).unwrap_or_default();
                    for (c, cell) in row.children.iter().enumerate() {
                        let pos = if r == 0 {
                            format!("table header; columns: {}", header.iter().map(|h| format!("\"{h}\"")).collect::<Vec<_>>().join(", "))
                        } else {
                            let row_part = if c > 0 && !row_label.is_empty() { format!(", row \"{row_label}\"") } else { String::new() };
                            format!("table column \"{}\"{row_part}", header.get(c).map(String::as_str).unwrap_or(""))
                        };
                        self.set_structure(local, Some(pos));
                        self.prose(&cell.children, SegmentKind::Cell, if r == 0 { "table header cell" } else { "table cell" });
                    }
                }
            }
            Kind::Paragraph { directive_label } => {
                self.set_structure(local, directive_label.then(|| "admonition title".to_string()));
                self.prose(&node.children, SegmentKind::Paragraph, where_);
            }
            Kind::Heading { depth } => {
                let d = *depth as usize;
                self.headings.resize(d - 1, None);
                self.set_structure(local, None);
                self.prose(&node.children, SegmentKind::Heading, &format!("heading level {d}"));
                self.headings.push(Some(to_string(node)));
            }
            Kind::Html { value } => {
                self.set_structure(local, None);
                process_html_block(self.ctx, node.start, node.end, value)?;
            }
            Kind::Code { value, lang, .. } => {
                if !self.ctx.options.code_comments {
                    return Ok(());
                }
                self.set_structure(local, lang.as_ref().filter(|l| !l.is_empty()).map(|l| format!("{l} code block")));
                process_code_block(self.ctx, node.start, node.end, lang.as_deref(), value);
            }
            Kind::Yaml { value } => {
                let translate = self.ctx.options.front_matter;
                self.frontmatter_formality = process_frontmatter(self.ctx, node.start, node.end, value, translate);
            }
            Kind::Definition { title, .. } => {
                let s = node.start;
                let raw = &src[s..node.end];
                self.definitions.push(raw.to_string());
                if title.is_some() {
                    if let Some(m) = DEFINITION_TITLE().exec(raw) {
                        self.set_structure(local, None);
                        let (prefix, quote, text) = (m.g(1), m.g(2), m.g(3));
                        let mut mb = self.ctx.builder();
                        mb.source(text, EscapeMode::Markdown);
                        let mut o = SegmentOptions::new(SegmentKind::Title, TextContext::MdTitle, text, "link title (tooltip)");
                        o.quote = quote.chars().next();
                        if let Some(seg) = self.ctx.segment(mb, o) {
                            let at = s + prefix.len() + 1;
                            self.ctx.replace(at, at + text.len(), seg);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn extract(input: &str, do_not_translate: &[String], options: ExtractOptions) -> Result<Extraction, String> {
    let bom = if input.starts_with('\u{FEFF}') { "\u{FEFF}" } else { "" };
    let source = &input[bom.len()..];
    let eol = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let parse_options = options.parse;
    let tree = parse_markdown(source, parse_options)?;
    let mut ctx = ExtractContext::new(source, do_not_translate, eol, options);
    let (definitions, frontmatter_formality) = {
        let mut x = Extractor { ctx: &mut ctx, definitions: Vec::new(), frontmatter_formality: None, headings: Vec::new() };
        x.visit(&tree, "paragraph", &[])?;
        (x.definitions, x.frontmatter_formality)
    };
    let mut replacements = std::mem::take(&mut ctx.replacements);
    replacements.sort_by_key(|r| r.start);
    for w in replacements.windows(2) {
        if w[1].start < w[0].end {
            return Err(format!("overlapping replacements at {}", w[1].start));
        }
    }
    Ok(Extraction {
        source: source.to_string(),
        bom: bom.to_string(),
        eol: eol.to_string(),
        store: ctx.store,
        replacements,
        notes: ctx.notes,
        definitions_text: definitions.join("\n"),
        frontmatter_formality,
        parse_options,
    })
}
