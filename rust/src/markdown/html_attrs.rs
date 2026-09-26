//! HTML attribute helpers and element tables. Port of `htmlAttrs.ts`.

use crate::regexutil::RegexExt;
use crate::rx;

/// Attributes whose values are human-readable text and get translated.
pub const TRANSLATABLE_ATTRS: &[&str] = &["alt", "title", "aria-label", "aria-description", "aria-placeholder", "placeholder", "label", "summary", "abbr"];

/// Props of MDX/JSX components that carry prose (compared lower-cased).
pub const TRANSLATABLE_JSX_PROPS: &[&str] = &[
    "alt", "title", "aria-label", "aria-description", "aria-placeholder", "placeholder", "label", "summary", "abbr", "description", "caption", "heading",
    "subtitle", "tooltip", "message", "emptytext", "helptext", "hint", "buttontext", "linktext",
];

/// Elements whose content is never translated.
pub const NO_TRANSLATE_ELEMENTS: &[&str] =
    &["script", "style", "pre", "code", "kbd", "samp", "var", "tt", "textarea", "svg", "math", "template", "noscript", "iframe", "object"];

pub const VOID_ELEMENTS: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];

pub const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "br", "cite", "code", "data", "del", "dfn", "em", "font", "i", "img", "ins", "kbd", "label", "mark", "q", "s", "samp",
    "small", "span", "strike", "strong", "sub", "sup", "time", "tt", "u", "var", "wbr", "big", "nobr", "picture", "source",
];

/// Which attribute set to translate in `tag_with_attributes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrSet {
    Html,
    Jsx,
}

impl AttrSet {
    pub fn contains(self, lower_name: &str) -> bool {
        match self {
            AttrSet::Html => TRANSLATABLE_ATTRS.contains(&lower_name),
            AttrSet::Jsx => TRANSLATABLE_JSX_PROPS.contains(&lower_name),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrValue {
    pub name: String,
    /// Byte offsets of the value (without quotes) relative to the tag string.
    pub start: usize,
    pub end: usize,
    pub quote: Option<char>,
}

rx!(ATTR, r#"[\s/]([A-Za-z_:@][-\w:.@]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?"#);
rx!(TAG_NAME, r"^<\/?([A-Za-z][A-Za-z0-9-]*)");
rx!(SELF_CLOSING, r"\/\s*>$");
rx!(NO_TRANSLATE_ATTR, r#"\stranslate\s*=\s*["']?no\b"#, "i");
rx!(NO_TRANSLATE_CLASS, r#"\sclass\s*=\s*["'][^"']*\bnotranslate\b"#, "i");

pub fn tag_name(tag: &str) -> Option<String> {
    TAG_NAME().exec(tag).map(|m| m.g(1).to_ascii_lowercase())
}

pub fn is_closing_tag(tag: &str) -> bool {
    tag.starts_with("</")
}

pub fn is_self_closing(tag: &str) -> bool {
    SELF_CLOSING().test(tag)
}

pub fn parse_attributes(tag: &str) -> Vec<AttrValue> {
    let name_end = TAG_NAME().exec(tag).map(|m| m.len()).unwrap_or(0);
    let body = &tag[name_end..];
    let mut out = Vec::new();
    for m in ATTR().all(body) {
        let name = m.g(1).to_ascii_lowercase();
        let (value, quote) = if let Some(v) = m.group(2) {
            (v, Some('"'))
        } else if let Some(v) = m.group(3) {
            (v, Some('\''))
        } else if let Some(v) = m.group(4) {
            (v, None)
        } else {
            continue;
        };
        let value_end_in_raw = m.len() - usize::from(quote.is_some());
        let value_start_in_raw = value_end_in_raw - value.len();
        let base = name_end + m.start;
        out.push(AttrValue { name, start: base + value_start_in_raw, end: base + value_end_in_raw, quote });
    }
    out
}

pub fn has_no_translate_marker(tag: &str) -> bool {
    NO_TRANSLATE_ATTR().test(tag) || NO_TRANSLATE_CLASS().test(tag)
}

/// Attribute values stripped of translatable text, for structural comparisons.
pub fn tag_skeleton(tag: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    for a in parse_attributes(tag) {
        if !TRANSLATABLE_ATTRS.contains(&a.name.as_str()) {
            continue;
        }
        out.push_str(&tag[pos..a.start]);
        out.push('…');
        pos = a.end;
    }
    out.push_str(&tag[pos..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes() {
        let tag = r#"<img src="a.png" alt="Company logo" width=120 title='T'>"#;
        let a = parse_attributes(tag);
        let alt = a.iter().find(|x| x.name == "alt").unwrap();
        assert_eq!(&tag[alt.start..alt.end], "Company logo");
        let w = a.iter().find(|x| x.name == "width").unwrap();
        assert_eq!((&tag[w.start..w.end], w.quote), ("120", None));
        assert_eq!(tag_skeleton(tag), r#"<img src="a.png" alt="…" width=120 title='…'>"#);
    }

    #[test]
    fn markers() {
        assert!(has_no_translate_marker(r#"<span translate="no">"#));
        assert!(has_no_translate_marker(r#"<p class="a notranslate">"#));
        assert!(!has_no_translate_marker("<p>"));
        assert_eq!(tag_name("</Div>").as_deref(), Some("div"));
        assert!(is_self_closing("<br />"));
    }
}
