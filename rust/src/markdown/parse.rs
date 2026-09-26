//! Markdown parsing: markdown-rs with GFM, front matter, math and (for `.mdx`) MDX, plus block directives
//! and optional single-dollar math. Port of `parse.ts`.

use super::ast::{self, Kind, Node};
use super::directive;
use super::math_spans::find_dollar_math;
use super::mdx::{esm_signal, expression_signal};
use crate::types::ParseOptions;
use markdown::{MdxExpressionKind, MdxExpressionParse, MdxSignal};
use std::cell::Cell;
use std::rc::Rc;

/// With `invalid`, an MDX expression that does not parse is accepted and recorded there instead of failing.
fn md_options(opts: ParseOptions, frontmatter: bool, invalid: Option<Rc<Cell<bool>>>) -> markdown::ParseOptions {
    let mut constructs = markdown::Constructs { frontmatter, math_flow: true, math_text: true, ..markdown::Constructs::gfm() };
    if opts.mdx {
        // micromark-extension-mdxjs disables these constructs.
        constructs.autolink = false;
        constructs.code_indented = false;
        constructs.html_flow = false;
        constructs.html_text = false;
        constructs.mdx_esm = true;
        constructs.mdx_expression_flow = true;
        constructs.mdx_expression_text = true;
        constructs.mdx_jsx_flow = true;
        constructs.mdx_jsx_text = true;
    }
    let expression_parse: Option<Box<MdxExpressionParse>> = match (opts.mdx, invalid) {
        (false, _) => None,
        (true, None) => Some(Box::new(expression_signal)),
        (true, Some(invalid)) => Some(Box::new(move |value: &str, kind: &MdxExpressionKind| match expression_signal(value, kind) {
            MdxSignal::Error(..) => {
                invalid.set(true);
                MdxSignal::Ok
            }
            signal => signal,
        })),
    };
    markdown::ParseOptions {
        constructs,
        gfm_strikethrough_single_tilde: true,
        math_text_single_dollar: false,
        mdx_esm_parse: if opts.mdx { Some(Box::new(esm_signal)) } else { None },
        mdx_expression_parse: expression_parse,
    }
}

fn to_node(text: &str, options: &markdown::ParseOptions) -> Result<Node, String> {
    let tree = markdown::to_mdast(text, options).map_err(|m| m.to_string())?;
    let mut node = ast::convert(&tree);
    ast::fix_tree(&mut node, text);
    Ok(node)
}

/// Parses without directives and without dollar-math splitting; positions refer to `text`.
pub fn parse_raw(text: &str, opts: ParseOptions, frontmatter: bool) -> Result<Node, String> {
    to_node(text, &md_options(opts, frontmatter, None))
}

pub fn parse_markdown(text: &str, opts: ParseOptions) -> Result<Node, String> {
    // Directive fences are found on a first pass. Their attribute blocks (`::video{#id}`, `:::note{.tip}`) look like
    // MDX expressions to markdown-rs, but in micromark the directive construct owns them. So the first pass accepts
    // invalid expressions; the masked text is parsed again strictly, and without fences the text is.
    let invalid = Rc::new(Cell::new(false));
    let first = to_node(text, &md_options(opts, true, Some(invalid.clone())))?;
    let mut excluded: Vec<usize> = Vec::new();
    let mut directed = None;
    for _ in 0..4 {
        let fences = directive::plan(text, &first, &excluded);
        if fences.is_empty() {
            break;
        }
        let masked = directive::mask(text, &fences);
        let mut second = parse_raw(&masked, opts, true)?;
        let breaks = directive::break_starts(&second);
        let missing: Vec<usize> = fences.iter().filter(|f| !breaks.iter().any(|b| f.is_break_start(*b))).map(|f| f.start).collect();
        if !missing.is_empty() {
            excluded.extend(missing);
            continue;
        }
        let definitions = definitions_text(&second, text);
        directive::Restructure { src: text, fences: &fences, opts, definitions }.run(&mut second);
        directed = Some(second);
        break;
    }
    let mut tree = match directed {
        Some(tree) => tree,
        None if invalid.get() => parse_raw(text, opts, true)?,
        None => first,
    };
    if opts.math_single_dollar {
        split_dollar_math(&mut tree, text);
    }
    Ok(tree)
}

fn definitions_text(tree: &Node, src: &str) -> String {
    let mut out = Vec::new();
    ast::walk(tree, &mut |n| match &n.kind {
        Kind::Definition { .. } => out.push(src[n.start..n.end].to_string()),
        Kind::FootnoteDefinition { identifier, label } => out.push(format!("[^{}]: x", label.as_deref().unwrap_or(identifier))),
        _ => {}
    });
    out.join("\n")
}

/// Turns `$...$` spans inside text nodes into inlineMath nodes (see `math_spans` for the rules).
fn split_dollar_math(node: &mut Node, src: &str) {
    if node.children.is_empty() {
        return;
    }
    let mut out = Vec::with_capacity(node.children.len());
    for mut c in std::mem::take(&mut node.children) {
        if !c.is_text() || c.end > src.len() || c.start > c.end {
            split_dollar_math(&mut c, src);
            out.push(c);
            continue;
        }
        let s = c.start;
        let raw = &src[s..c.end];
        let spans = find_dollar_math(raw);
        if spans.is_empty() {
            out.push(c);
            continue;
        }
        let mut pos = 0;
        for (a, b) in spans {
            if a > pos {
                out.push(Node::new(Kind::Text { value: raw[pos..a].to_string() }, s + pos, s + a));
            }
            out.push(Node::new(Kind::InlineMath { value: raw[a + 1..b - 1].to_string() }, s + a, s + b));
            pos = b;
        }
        if pos < raw.len() {
            out.push(Node::new(Kind::Text { value: raw[pos..].to_string() }, s + pos, s + raw.len()));
        }
    }
    node.children = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(n: &Node, depth: usize, out: &mut Vec<String>) {
        let extra = match &n.kind {
            Kind::ContainerDirective { name, .. } | Kind::LeafDirective { name, .. } => format!(" {name}"),
            Kind::Text { value } => format!(" {value:?}"),
            _ => String::new(),
        };
        out.push(format!("{}{} {}-{}{}", "  ".repeat(depth), n.type_name(), n.start, n.end, extra));
        for c in &n.children {
            types(c, depth + 1, out);
        }
    }

    fn dump(src: &str) -> String {
        let t = parse_markdown(src, ParseOptions::default()).unwrap();
        let mut out = Vec::new();
        types(&t, 0, &mut out);
        out.join("\n")
    }

    #[test]
    fn container_directive_with_list() {
        let d = dump(":::note\n- a\n- b\n:::\n\nafter\n");
        assert!(d.contains("containerDirective 0-19 note"), "{d}");
        assert!(d.contains("paragraph 21-26"), "{d}");
    }

    #[test]
    fn nested_directives() {
        let d = dump("::::tabs\n:::tab[Windows]\nOpen **Start**.\n:::\n:::tab[macOS]\nOpen it.\n:::\n::::\n");
        assert_eq!(d.matches("containerDirective").count(), 3, "{d}");
        assert!(d.contains("text 16-23 \"Windows\""), "{d}");
    }

    #[test]
    fn leaf_directive() {
        let d = dump("::youtube[Watch *this*]{#id}\n");
        assert!(d.contains("leafDirective 0-28 youtube"), "{d}");
        assert!(d.contains("text 10-16 \"Watch \""), "{d}");
        assert!(d.contains("emphasis 16-22"), "{d}");
    }

    #[test]
    fn invalid_fences_stay_text() {
        let d = dump(":::tip Title\ncontent\n:::\n");
        assert!(!d.contains("containerDirective"), "{d}");
        let d = dump("```md\n:::note\n```\n");
        assert!(!d.contains("containerDirective"), "{d}");
    }

    #[test]
    fn dollar_math() {
        let t = parse_markdown("Let $x^2$ cost $5.\n", ParseOptions { math_single_dollar: true, mdx: false }).unwrap();
        let p = &t.children[0];
        assert_eq!(p.children.iter().map(|c| c.type_name()).collect::<Vec<_>>(), vec!["text", "inlineMath", "text"]);
    }

    #[test]
    fn directive_attributes_are_not_mdx_expressions() {
        let mdx = ParseOptions { mdx: true, math_single_dollar: false };
        let t = parse_markdown("::video[Intro]{#abc}\n\n:::note{.tip}\nText\n:::\n\n::file{src=\"a.zip\" label=\"A\"}\n", mdx).unwrap();
        let kinds: Vec<&str> = t.children.iter().map(|c| c.type_name()).collect();
        assert_eq!(kinds, vec!["leafDirective", "containerDirective", "leafDirective"]);
        // Invalid expressions outside directive fences still fail, as in micromark.
        let err = parse_markdown("Text {#abc}\n", mdx).unwrap_err();
        assert!(err.contains("Could not parse expression with acorn"), "{err}");
        assert!(parse_markdown("::video[Intro]{#abc}\n\nText {#abc}\n", mdx).is_err());
        assert!(parse_markdown(":::note\nText {.tip}\n:::\n", mdx).is_err());
    }
}
