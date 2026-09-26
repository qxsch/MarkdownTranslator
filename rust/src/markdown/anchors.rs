//! Keeps links to the original heading anchors working after headings were translated. Port of
//! `markdown/anchors.ts`, with github-slugger's algorithm.

use super::ast::{JsxAttr, Kind, Node};
use super::parse::parse_markdown;
use super::slug_table::REMOVED;
use crate::regexutil::RegexExt;
use crate::rx;
use crate::types::ParseOptions;
use std::collections::HashMap;

fn removed(c: char) -> bool {
    let cp = c as u32;
    REMOVED.binary_search_by(|(a, b)| if cp < *a { std::cmp::Ordering::Greater } else if cp > *b { std::cmp::Ordering::Less } else { std::cmp::Ordering::Equal }).is_ok()
}

/// github-slugger's `slug(value)`.
pub fn slugify(value: &str) -> String {
    value.to_lowercase().chars().filter(|c| !removed(*c)).map(|c| if c == ' ' { '-' } else { c }).collect()
}

/// github-slugger's `BananaSlug`: repeated values get `-1`, `-2`... suffixes.
#[derive(Default)]
pub struct Slugger {
    occurrences: HashMap<String, usize>,
}

impl Slugger {
    pub fn slug(&mut self, value: &str) -> String {
        let original = slugify(value);
        let mut result = original.clone();
        while self.occurrences.contains_key(&result) {
            let n = self.occurrences.get_mut(&original).expect("original slug is tracked");
            *n += 1;
            result = format!("{original}-{n}");
        }
        self.occurrences.insert(result.clone(), 0);
        result
    }
}

struct HeadingInfo {
    text: String,
    content_start: usize,
    has_explicit_id: bool,
}

fn text_of(n: &Node) -> String {
    match &n.kind {
        Kind::Text { value } | Kind::InlineCode { value } => value.clone(),
        Kind::Html { .. } | Kind::Image { .. } | Kind::ImageReference { .. } => String::new(),
        _ => n.children.iter().map(text_of).collect(),
    }
}

rx!(ANCHOR_WITH_ID, r"<a\s[^>]*\b(?:id|name)\s*=", "i");
rx!(CUSTOM_ID, r"\{#[\w-]+\}\s*$");

fn headings(text: &str, opts: ParseOptions) -> Result<Vec<HeadingInfo>, String> {
    let tree = parse_markdown(text, opts)?;
    let mut out = Vec::new();
    fn visit(n: &Node, out: &mut Vec<HeadingInfo>) {
        if let Kind::Heading { .. } = n.kind {
            if n.children.is_empty() {
                return;
            }
            let plain = text_of(n);
            let explicit = n.children.iter().any(|c| match &c.kind {
                Kind::Html { value } => ANCHOR_WITH_ID().test(value),
                Kind::MdxJsxTextElement { name: Some(name), attributes } if name == "a" => {
                    attributes.iter().any(|a| matches!(a, JsxAttr::Property { name, .. } if name == "id" || name == "name"))
                }
                _ => false,
            }) || CUSTOM_ID().test(&plain);
            out.push(HeadingInfo { text: plain, content_start: n.children[0].start, has_explicit_id: explicit });
            return;
        }
        for c in &n.children {
            visit(c, out);
        }
    }
    visit(&tree, &mut out);
    Ok(out)
}

/// Inserts an empty HTML anchor with the original slug at the start of each changed heading. The anchor has
/// no text, so it does not change the new heading's own slug.
pub fn preserve_anchors(source: &str, output: &str, opts: ParseOptions) -> Result<(String, Vec<String>), String> {
    let a = headings(source, opts)?;
    let b = headings(output, opts)?;
    if a.len() != b.len() {
        return Ok((output.to_string(), Vec::new()));
    }
    let mut old = Slugger::default();
    let mut new = Slugger::default();
    let mut inserts: Vec<(usize, String, String)> = Vec::new();
    for (x, y) in a.iter().zip(b.iter()) {
        let old_slug = old.slug(&x.text);
        let new_slug = new.slug(&y.text);
        if !old_slug.is_empty() && old_slug != new_slug && !y.has_explicit_id && !x.has_explicit_id {
            inserts.push((y.content_start, format!("<a id=\"{old_slug}\"></a>"), old_slug));
        }
    }
    let mut text = output.to_string();
    let mut sorted: Vec<&(usize, String, String)> = inserts.iter().collect();
    sorted.sort_by(|p, q| q.0.cmp(&p.0));
    for (at, t, _) in sorted {
        text.insert_str(*at, t);
    }
    Ok((text, inserts.into_iter().map(|i| i.2).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_match_github() {
        assert_eq!(slugify("Getting started with `mdtranslator`"), "getting-started-with-mdtranslator");
        assert_eq!(slugify("Über uns & mehr!"), "über-uns--mehr");
        assert_eq!(slugify("snake_case — dash"), "snake_case--dash");
        let mut s = Slugger::default();
        assert_eq!((s.slug("A"), s.slug("A"), s.slug("A")), ("a".into(), "a-1".into(), "a-2".into()));
    }
}
