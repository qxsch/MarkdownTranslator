//! Shared state while extracting segments from one document. Port of `context.ts`.

use super::html_attrs::{parse_attributes, AttrSet};
use crate::mask::masking::{EscapeMode, MaskBuilder};
use crate::regexutil::has_letter;
use crate::types::{ExtractOptions, Finalize, Render, Replacement, Segment, SegmentKind, SegmentStore, TagPart, TextContext, WrapSpec};

pub struct SegmentOptions {
    pub kind: SegmentKind,
    pub text_context: TextContext,
    pub original: String,
    pub note: String,
    pub wrap: Option<WrapSpec>,
    pub forbidden: Vec<String>,
    pub quote: Option<char>,
    pub finalize: Finalize,
    pub embedded: bool,
    pub inline_signature: Option<String>,
    pub dependents: Vec<String>,
    /// Create the segment even without own text when it has dependents (it then becomes passive).
    pub force: bool,
}

impl SegmentOptions {
    pub fn new(kind: SegmentKind, text_context: TextContext, original: impl Into<String>, note: impl Into<String>) -> Self {
        SegmentOptions {
            kind,
            text_context,
            original: original.into(),
            note: note.into(),
            wrap: None,
            forbidden: Vec::new(),
            quote: None,
            finalize: Finalize::None,
            embedded: false,
            inline_signature: None,
            dependents: Vec::new(),
            force: false,
        }
    }
}

pub struct ExtractContext<'a> {
    pub source: &'a str,
    pub dnt: &'a [String],
    pub eol: String,
    pub options: ExtractOptions,
    pub store: SegmentStore,
    pub replacements: Vec<Replacement>,
    pub notes: Vec<String>,
    /// Structural position applied to segments created from now on.
    pub structure: Option<String>,
    counter: usize,
}

impl<'a> ExtractContext<'a> {
    pub fn new(source: &'a str, dnt: &'a [String], eol: &str, options: ExtractOptions) -> Self {
        ExtractContext {
            source,
            dnt,
            eol: eol.to_string(),
            options,
            store: SegmentStore::default(),
            replacements: Vec::new(),
            notes: Vec::new(),
            structure: None,
            counter: 0,
        }
    }

    pub fn builder(&self) -> MaskBuilder<'a> {
        MaskBuilder::new(self.dnt)
    }

    /// Creates a segment when the builder holds translatable text; returns its id.
    pub fn segment(&mut self, mb: MaskBuilder, o: SegmentOptions) -> Option<String> {
        let has_text = has_letter(&mb.source_text);
        if !has_text && !(o.force && !o.dependents.is_empty()) {
            return None;
        }
        self.counter += 1;
        let id = format!("s{}", self.counter);
        let masked = mb.masked().to_string();
        let seg = Segment {
            id: id.clone(),
            kind: o.kind,
            masked,
            placeholders: mb.placeholders,
            pairs: mb.pairs,
            text_context: o.text_context,
            original: o.original,
            source_text: mb.source_text,
            note: o.note,
            structure: self.structure.clone(),
            tag_groups: mb.tag_groups,
            wrap: o.wrap,
            forbidden: o.forbidden,
            quote: o.quote,
            finalize: o.finalize,
            inline_signature: o.inline_signature,
            embedded: o.embedded,
            dependents: o.dependents,
            passive: !has_text,
        };
        self.store.push(seg);
        Some(id)
    }

    pub fn replace(&mut self, start: usize, end: usize, seg: String) {
        self.replacements.push(Replacement { start, end, seg });
    }

    /// Makes translatable attributes (alt, title, aria-label...) of a single tag into embedded segments.
    /// Returns a render producing the tag with translated values, plus the embedded segment ids.
    pub fn tag_with_attributes(&mut self, tag: &str, note: &str, names: AttrSet) -> (Render, Vec<String>) {
        let mut parts = Vec::new();
        for a in parse_attributes(tag) {
            if !names.contains(&a.name.to_lowercase()) {
                continue;
            }
            let value = &tag[a.start..a.end];
            let mut mb = self.builder();
            mb.source(value, EscapeMode::Html);
            let mut o = SegmentOptions::new(
                if a.name == "alt" { SegmentKind::Alt } else { SegmentKind::Attr },
                TextContext::Attr,
                value,
                format!("{note}, HTML {} attribute", a.name),
            );
            o.quote = a.quote;
            o.embedded = true;
            o.finalize = if a.quote.is_some() { Finalize::None } else { Finalize::AttrUnquoted };
            if let Some(seg) = self.segment(mb, o) {
                parts.push(TagPart { start: a.start, end: a.end, seg });
            }
        }
        if parts.is_empty() {
            return (Render::literal(tag), Vec::new());
        }
        let ids = parts.iter().map(|p| p.seg.clone()).collect();
        (Render::Spliced { text: tag.to_string(), parts }, ids)
    }
}
