//! Core data model shared by the extraction, translation and assembly stages.
//!
//! The TypeScript original stores JavaScript closures on placeholders, pairs and replacements.
//! Rust cannot cheaply hold closures that borrow the segment store, so every "render" is expressed
//! as an owned [`Render`] tree that is resolved against the store when the document is assembled.

use std::collections::BTreeMap;
use std::collections::HashMap;

/// Translated masked text per segment id; a missing entry means "keep source".
pub type TMap = HashMap<String, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentKind {
    Paragraph,
    Heading,
    Cell,
    Label,
    Alt,
    Title,
    Attr,
    Html,
    Comment,
    Frontmatter,
}

impl SegmentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SegmentKind::Paragraph => "paragraph",
            SegmentKind::Heading => "heading",
            SegmentKind::Cell => "cell",
            SegmentKind::Label => "label",
            SegmentKind::Alt => "alt",
            SegmentKind::Title => "title",
            SegmentKind::Attr => "attr",
            SegmentKind::Html => "html",
            SegmentKind::Comment => "comment",
            SegmentKind::Frontmatter => "frontmatter",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextContext {
    Markdown,
    Cell,
    Html,
    Attr,
    MdTitle,
    Comment,
    Yaml,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairKind {
    Emphasis,
    Strong,
    Delete,
    Link,
    Image,
    Html,
}

impl PairKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PairKind::Emphasis => "emphasis",
            PairKind::Strong => "strong",
            PairKind::Delete => "delete",
            PairKind::Link => "link",
            PairKind::Image => "image",
            PairKind::Html => "html",
        }
    }

    pub fn is_emphasis(self) -> bool {
        matches!(self, PairKind::Emphasis | PairKind::Strong | PairKind::Delete)
    }
}

/// A translatable value embedded inside literal bytes (e.g. an `alt="…"` attribute inside a tag).
#[derive(Clone, Debug)]
pub struct TagPart {
    /// Byte range of the value inside the owning literal.
    pub start: usize,
    pub end: usize,
    pub seg: String,
}

/// Owned replacement for the TypeScript `(tm) => string` render closures.
#[derive(Clone, Debug)]
pub enum Render {
    /// Constant bytes.
    Literal(String),
    /// Literal bytes with embedded segments spliced into the given ranges.
    Spliced { text: String, parts: Vec<TagPart> },
    /// `before` + rendered segment + `after`; used for link/image titles.
    Wrapped {
        before: String,
        seg: String,
        after: String,
    },
}

impl Render {
    pub fn literal(s: impl Into<String>) -> Render {
        Render::Literal(s.into())
    }

    /// Source bytes of the render, ignoring any translation (used for hints and fallbacks).
    pub fn raw(&self) -> &str {
        match self {
            Render::Literal(s) => s,
            Render::Spliced { text, .. } => text,
            Render::Wrapped { before, .. } => before,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Placeholder {
    pub n: u32,
    /// Short preview of what the placeholder stands for, shown to the model for grammar context.
    pub hint: String,
    pub render: Render,
    /// True when the rendered bytes contain a line break (hard break), relevant for wrapping.
    pub hard_break: bool,
}

#[derive(Clone, Debug)]
pub struct Pair {
    pub n: u32,
    pub kind: PairKind,
    pub hint: String,
    pub open: Render,
    pub close: Render,
    /// For `[label]` / `[label][]` references: original label and closing bytes, kept when the label is unchanged.
    pub ref_label: Option<String>,
    pub close_original: Option<String>,
    /// Quote character of the attribute value this pair wraps; it must not appear in the translation.
    pub quote: Option<char>,
}

/// Tags (e.g. "x3", "g4") whose order must survive translation; contiguous groups also allow no text between them.
#[derive(Clone, Debug)]
pub struct TagGroup {
    pub tokens: Vec<String>,
    pub contiguous: bool,
}

#[derive(Clone, Debug)]
pub struct WrapSpec {
    /// Maximum line width observed in the source block, in characters.
    pub width: usize,
    /// Column where the first line of the block starts, in characters.
    pub first_column: usize,
    /// Bytes emitted after each inserted line break (container prefix / indentation).
    pub prefix: String,
    pub eol: String,
}

/// Final transformation over the rendered text (replaces the TypeScript `finalize` closure).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finalize {
    None,
    /// YAML double-quoted scalar.
    YamlDouble,
    /// YAML single-quoted scalar.
    YamlSingle,
    /// YAML plain scalar, double-quoted only when the value would otherwise change meaning.
    YamlPlain,
    /// Unquoted HTML attribute value, quoted only when it contains characters that need it.
    AttrUnquoted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formality {
    Formal,
    Informal,
}

impl Formality {
    pub fn as_str(self) -> &'static str {
        match self {
            Formality::Formal => "formal",
            Formality::Informal => "informal",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub id: String,
    pub kind: SegmentKind,
    /// Source text with inline structure replaced by `<gN>…</gN>` and `<xN/>` tags; text is XML-escaped.
    pub masked: String,
    pub placeholders: BTreeMap<u32, Placeholder>,
    pub pairs: BTreeMap<u32, Pair>,
    pub text_context: TextContext,
    /// Original bytes of the segment; returned verbatim when the segment is untranslated.
    pub original: String,
    /// Plain source text outside of tags, used to decide which markup characters need escaping.
    pub source_text: String,
    /// Human-readable context passed to the model (e.g. "table cell", "python comment").
    pub note: String,
    /// Position in the document (section path, table column/row, list lead-in), optional model context.
    pub structure: Option<String>,
    /// Tag order constraints (fence lines, attribute values inside shortcodes).
    pub tag_groups: Vec<TagGroup>,
    pub wrap: Option<WrapSpec>,
    /// Substrings that must not appear in translated text (e.g. comment terminators).
    pub forbidden: Vec<String>,
    /// Quote character for attribute/title contexts.
    pub quote: Option<char>,
    pub finalize: Finalize,
    /// Inline signature of the source (for prose kinds) used for structural validation.
    pub inline_signature: Option<String>,
    /// Segments that are rendered inside another segment's tag instead of at their own range.
    pub embedded: bool,
    /// Ids of embedded segments rendered inside this segment.
    pub dependents: Vec<String>,
    /// Passive segments carry no translatable text of their own and are never sent to a translator.
    pub passive: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOptions {
    /// Re-wrap the rendered block to the source width. Defaults to off; [`RenderOptions::wrapped`] turns it on.
    pub wrap: bool,
    /// Render through the placeholder path even for untranslated segments (tests byte fidelity).
    pub force: bool,
}

impl RenderOptions {
    pub fn wrapped(wrap: bool) -> RenderOptions {
        RenderOptions { wrap, force: false }
    }
}

#[derive(Clone, Debug)]
pub struct Replacement {
    pub start: usize,
    pub end: usize,
    pub seg: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// Parse as MDX (JSX, ESM, expressions); used for `.mdx` files.
    pub mdx: bool,
    /// Parse `$x$` as inline math (Pandoc rules plus a formula check, so prices stay prose).
    pub math_single_dollar: bool,
}

/// Owned store of every segment produced for one document.
#[derive(Clone, Debug, Default)]
pub struct SegmentStore {
    pub segments: Vec<Segment>,
    index: HashMap<String, usize>,
}

impl SegmentStore {
    pub fn push(&mut self, seg: Segment) {
        self.index.insert(seg.id.clone(), self.segments.len());
        self.segments.push(seg);
    }

    pub fn get(&self, id: &str) -> Option<&Segment> {
        self.index.get(id).map(|&i| &self.segments[i])
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Segment> {
        self.segments.iter()
    }
}

#[derive(Clone, Debug)]
pub struct Extraction {
    pub source: String,
    pub bom: String,
    pub eol: String,
    pub store: SegmentStore,
    pub replacements: Vec<Replacement>,
    pub notes: Vec<String>,
    /// Link/footnote definitions, appended when validating isolated inline content.
    pub definitions_text: String,
    pub frontmatter_formality: Option<Formality>,
    pub parse_options: ParseOptions,
}

impl Extraction {
    pub fn segments(&self) -> &[Segment] {
        &self.store.segments
    }

    pub fn get(&self, id: &str) -> Option<&Segment> {
        self.store.get(id)
    }

    /// Segments that carry their own translatable text.
    pub fn active(&self) -> impl Iterator<Item = &Segment> {
        self.store.iter().filter(|s| !s.passive)
    }
}

/// Feature switches resolved for a single document; mirrors `TranslateOptions` in the TypeScript code.
#[derive(Clone, Debug)]
pub struct ExtractOptions {
    pub parse: ParseOptions,
    /// Translate Python docstrings.
    pub docstrings: bool,
    /// Translate comments in fenced code blocks.
    pub code_comments: bool,
    /// Translate prose values in YAML front matter.
    pub front_matter: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        ExtractOptions {
            parse: ParseOptions::default(),
            docstrings: true,
            code_comments: true,
            front_matter: true,
        }
    }
}
