//! YAML front matter model built on saphyr-parser events: scalar styles and byte ranges (like the `yaml`
//! package's `node.range`), YAML 1.2 core schema typing, and a masked value for structure comparisons.

use saphyr_parser::{Event, Parser, ScalarStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    /// Integer or float, kept as written.
    Number(String),
    Str(String),
    Seq(Vec<YNode>),
    Map(Vec<(YNode, YNode)>),
    /// An alias; `resolved` is the anchored node (for the skeleton).
    Alias(Box<YNode>),
}

#[derive(Clone, Debug)]
pub struct YNode {
    pub value: Value,
    /// Scalar style (None for collections and aliases).
    pub style: Option<Style>,
    /// Byte range: for block scalars from the `|`/`>` indicator, for others the node itself.
    pub start: usize,
    pub end: usize,
}

impl YNode {
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_scalar(&self) -> bool {
        self.style.is_some()
    }

    /// `String(scalar.value)` for keys.
    pub fn key_string(&self) -> String {
        match &self.value {
            Value::Null => "null".into(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.clone(),
            Value::Str(s) => s.clone(),
            _ => String::new(),
        }
    }
}

/// YAML 1.2 core schema resolution of a plain scalar.
fn resolve_plain(v: &str) -> Value {
    use std::sync::OnceLock;
    static INT: OnceLock<regex::Regex> = OnceLock::new();
    static FLOAT: OnceLock<regex::Regex> = OnceLock::new();
    let int = INT.get_or_init(|| regex::Regex::new(r"^(?:[-+]?[0-9]+|0o[0-7]+|0x[0-9a-fA-F]+)$").unwrap());
    let float = FLOAT.get_or_init(|| {
        regex::Regex::new(r"^(?:[-+]?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)(?:[eE][-+]?[0-9]+)?|[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$").unwrap()
    });
    match v {
        "" | "~" | "null" | "Null" | "NULL" => Value::Null,
        "true" | "True" | "TRUE" => Value::Bool(true),
        "false" | "False" | "FALSE" => Value::Bool(false),
        _ if int.is_match(v) || float.is_match(v) => Value::Number(v.to_string()),
        _ => Value::Str(v.to_string()),
    }
}

enum Frame {
    Seq { items: Vec<YNode>, start: usize, anchor: usize },
    Map { pairs: Vec<(YNode, YNode)>, pending: Option<YNode>, start: usize, anchor: usize },
}

/// Position of the `|`/`>` indicator of a block scalar whose content starts at `content`: the first
/// indicator character after `lower_bound` (the previous token) on the last non-blank line before it.
fn block_indicator(text: &str, lower_bound: usize, content: usize) -> usize {
    let content = content.min(text.len());
    let mut line_start = text[..content].rfind('\n').map(|i| i + 1).unwrap_or(0);
    while line_start > 0 {
        let prev_end = line_start - 1;
        let prev_start = text[..prev_end].rfind('\n').map(|i| i + 1).unwrap_or(0);
        if !text[prev_start..prev_end].trim().is_empty() {
            let from = prev_start.max(lower_bound).min(prev_end);
            return match text[from..prev_end].find(['|', '>']) {
                Some(p) => from + p,
                None => lower_bound.min(content),
            };
        }
        line_start = prev_start;
    }
    lower_bound.min(content)
}

pub fn parse(text: &str) -> Result<YNode, String> {
    let char_to_byte: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
    let at = |i: usize| char_to_byte.get(i).copied().unwrap_or(text.len());
    let mut stack: Vec<Frame> = Vec::new();
    let mut anchors: std::collections::HashMap<usize, YNode> = std::collections::HashMap::new();
    let mut root: Option<YNode> = None;
    let mut last_end = 0usize;
    let mut documents = 0;

    fn push(stack: &mut Vec<Frame>, root: &mut Option<YNode>, node: YNode) -> Result<(), String> {
        match stack.last_mut() {
            None => {
                *root = Some(node);
                Ok(())
            }
            Some(Frame::Seq { items, .. }) => {
                items.push(node);
                Ok(())
            }
            Some(Frame::Map { pairs, pending, .. }) => {
                match pending.take() {
                    None => *pending = Some(node),
                    Some(key) => {
                        if key.is_scalar() && pairs.iter().any(|(k, _)| k.is_scalar() && k.key_string() == key.key_string()) {
                            return Err("Map keys must be unique".into());
                        }
                        pairs.push((key, node));
                    }
                }
                Ok(())
            }
        }
    }

    for ev in Parser::new_from_str(text) {
        let (event, span) = ev.map_err(|e| e.to_string())?;
        let (s, e) = (at(span.start.index()), at(span.end.index()));
        match event {
            Event::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    return Err("Source contains multiple documents".into());
                }
            }
            Event::Scalar(v, style, anchor, tag) => {
                let tagged_str = tag.as_ref().is_some_and(|t| t.suffix == "str");
                let (value, st) = match style {
                    ScalarStyle::Plain if !tagged_str && tag.is_none() => (resolve_plain(&v), Style::Plain),
                    ScalarStyle::Plain => (Value::Str(v.to_string()), Style::Plain),
                    ScalarStyle::SingleQuoted => (Value::Str(v.to_string()), Style::SingleQuoted),
                    ScalarStyle::DoubleQuoted => (Value::Str(v.to_string()), Style::DoubleQuoted),
                    ScalarStyle::Literal => (Value::Str(v.to_string()), Style::Literal),
                    ScalarStyle::Folded => (Value::Str(v.to_string()), Style::Folded),
                };
                let start = if matches!(st, Style::Literal | Style::Folded) { block_indicator(text, last_end, s) } else { s };
                let node = YNode { value, style: Some(st), start, end: e.max(start) };
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                last_end = e;
                push(&mut stack, &mut root, node)?;
            }
            Event::Alias(id) => {
                let target = anchors.get(&id).cloned().ok_or_else(|| "unknown alias".to_string())?;
                let node = YNode { value: Value::Alias(Box::new(target)), style: None, start: s, end: e };
                last_end = e;
                push(&mut stack, &mut root, node)?;
            }
            Event::SequenceStart(anchor, _) => {
                stack.push(Frame::Seq { items: Vec::new(), start: s, anchor });
                last_end = e;
            }
            Event::MappingStart(anchor, _) => {
                stack.push(Frame::Map { pairs: Vec::new(), pending: None, start: s, anchor });
                last_end = e;
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let frame = stack.pop().ok_or_else(|| "unbalanced collection".to_string())?;
                let (value, start, anchor) = match frame {
                    Frame::Seq { items, start, anchor } => (Value::Seq(items), start, anchor),
                    Frame::Map { pairs, start, anchor, .. } => (Value::Map(pairs), start, anchor),
                };
                let node = YNode { value, style: None, start, end: e.max(start) };
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                last_end = e;
                push(&mut stack, &mut root, node)?;
            }
            _ => {}
        }
    }
    Ok(root.unwrap_or(YNode { value: Value::Null, style: None, start: 0, end: 0 }))
}

/// Front matter keys (at any nesting depth) whose string values are prose.
pub const FRONTMATTER_KEYS: &[&str] = &[
    "title", "subtitle", "titlesuffix", "description", "summary", "excerpt", "abstract", "caption", "heading", "lead", "teaser", "tagline", "seo_title",
    "seotitle", "seo_description", "og_title", "og_description", "twitter_title", "twitter_description", "sidebar_label", "nav_title", "linktitle",
    "menu_title",
];

/// Front matter keys whose list items are prose.
pub const FRONTMATTER_LIST_KEYS: &[&str] = &["keywords"];

/// JSON-like rendering of the document with prose values masked (`yamlSkeleton` in `document.ts`).
pub fn skeleton(text: &str) -> String {
    match parse(text) {
        Ok(root) => {
            let mut out = String::new();
            masked(&root, &mut out);
            out
        }
        Err(_) => "invalid-yaml".into(),
    }
}

fn scalar_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(n),
        Value::Str(s) => out.push_str(&crate::jsstr::json_string(s)),
        _ => {}
    }
}

fn masked(n: &YNode, out: &mut String) {
    match &n.value {
        Value::Alias(target) => masked(target, out),
        Value::Seq(items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                masked(it, out);
            }
            out.push(']');
        }
        Value::Map(pairs) => {
            out.push('{');
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let key = k.key_string();
                out.push_str(&crate::jsstr::json_string(&key));
                out.push(':');
                let lower = key.to_lowercase();
                let target = match &v.value {
                    Value::Alias(t) => t.as_ref(),
                    _ => v,
                };
                if FRONTMATTER_KEYS.contains(&lower.as_str()) && target.as_str().is_some() {
                    out.push_str("\"…\"");
                } else if FRONTMATTER_LIST_KEYS.contains(&lower.as_str()) && matches!(target.value, Value::Seq(_)) {
                    let Value::Seq(items) = &target.value else { unreachable!() };
                    out.push('[');
                    for (j, x) in items.iter().enumerate() {
                        if j > 0 {
                            out.push(',');
                        }
                        if x.as_str().is_some() {
                            out.push_str("\"…\"");
                        } else {
                            masked(x, out);
                        }
                    }
                    out.push(']');
                } else {
                    masked(v, out);
                }
            }
            out.push('}');
        }
        v => scalar_json(v, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_styles() {
        let t = "title: Hello world # c\nd: \"Quoted\"\nf: >\n  folded\n  text\nn: 42\n";
        let root = parse(t).unwrap();
        let Value::Map(pairs) = &root.value else { panic!() };
        let v = &pairs[0].1;
        assert_eq!((&t[v.start..v.end], v.style), ("Hello world", Some(Style::Plain)));
        let q = &pairs[1].1;
        assert_eq!(&t[q.start..q.end], "\"Quoted\"");
        let f = &pairs[2].1;
        assert_eq!(f.style, Some(Style::Folded));
        assert!(t[f.start..].starts_with(">\n  folded"), "{:?}", &t[f.start..f.end]);
        assert!(matches!(pairs[3].1.value, Value::Number(_)));
    }

    #[test]
    fn errors() {
        assert!(parse("a: 1\na: 2\n").is_err());
        assert!(parse("a: [1\n").is_err());
    }

    #[test]
    fn masked_skeleton() {
        assert_eq!(skeleton("title: A\nkeywords: [x, 1]\nn: 2\n"), r#"{"title":"…","keywords":["…",1],"n":2}"#);
    }
}
