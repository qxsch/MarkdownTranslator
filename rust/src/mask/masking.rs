//! Masking of inline structure into `<gN>…</gN>` / `<xN/>` tags, tag validation, and rendering of translated
//! masked text back into source syntax. Port of `masking.ts`.

use super::protect::find_protected;
use crate::jsstr;
use crate::regexutil::{has_letter, RegexExt};
use crate::rx;
use crate::types::{Pair, PairKind, Placeholder, Render, Segment, SegmentStore, TMap, TagGroup, TextContext};
use std::collections::BTreeMap;

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

rx!(XML_ENTITY, r"&(lt|gt|quot|apos|amp);");

pub fn xml_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    XML_ENTITY().replace_all_with(s, |m| {
        match m.g(1) {
            "lt" => "<",
            "gt" => ">",
            "quot" => "\"",
            "apos" => "'",
            _ => "&",
        }
        .to_string()
    })
}

rx!(MD_ESCAPE, r"\\[!-/:-@[-`{-~]");
rx!(ENTITY, r"&(?:[A-Za-z][A-Za-z0-9]{1,31}|#\d{1,7}|#[xX][0-9a-fA-F]{1,6});");
// Line break inside running text plus the container prefix (blockquote markers / indentation) that follows it.
rx!(SOFT_BREAK, r"[ \t]*(\r?\n)((?:[ \t]*>)*[ \t]*)");
// Human-readable attribute values inside shortcodes and directive lines.
rx!(ATTR_VALUE, r#"\b(alt|alt-text|title|caption|label|summary)(\s*=\s*)(["'])(.*?)\3"#);
// Lines that are structure on their own: `:::note Title`, `:::image ... :::`, `:::`, `[!NOTE]`, `{{% notice %}}`.
rx!(FENCE_LINE, r"^(?::{3,}|\[![A-Za-z]+\]\s*$|\{\{[<%][^\n]*[>%]\}\}\s*$)");
rx!(ATTRIBUTE_ASSIGNMENT, r#"\w[\w-]*\s*=\s*["']"#);
rx!(FENCE_TITLE, r"^(:{3,}\s*[A-Za-z][\w-]*\s*)(.*)$");
rx!(TRAILING_FENCE, r":{3,}\s*$");
rx!(SHORTCODE_START, r"^\{\{[<%]");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscapeMode {
    Markdown,
    Html,
    None,
}

#[derive(Clone, Debug)]
pub struct SoftBreakInfo {
    pub eol: String,
    pub prefix: String,
}

pub struct MaskBuilder<'a> {
    parts: String,
    pub placeholders: BTreeMap<u32, Placeholder>,
    pub pairs: BTreeMap<u32, Pair>,
    pub tag_groups: Vec<TagGroup>,
    pub soft_break: Option<SoftBreakInfo>,
    n: u32,
    /// Plain (decoded) source text, used to decide which markup characters need escaping on output.
    pub source_text: String,
    dnt: &'a [String],
}

fn truncate(s: &str) -> String {
    let one = jsstr::collapse_ws(s);
    if jsstr::u16len(&one) > 80 {
        format!("{}...", jsstr::prefix_u16(&one, 77))
    } else {
        one
    }
}

impl<'a> MaskBuilder<'a> {
    pub fn new(dnt: &'a [String]) -> Self {
        MaskBuilder {
            parts: String::new(),
            placeholders: BTreeMap::new(),
            pairs: BTreeMap::new(),
            tag_groups: Vec::new(),
            soft_break: None,
            n: 0,
            source_text: String::new(),
            dnt,
        }
    }

    pub fn masked(&self) -> &str {
        &self.parts
    }

    pub fn text(&mut self, t: &str) {
        if t.is_empty() {
            return;
        }
        self.source_text.push_str(t);
        self.parts.push_str(&xml_escape(t));
    }

    pub fn placeholder(&mut self, render: Render, hint: &str, hard_break: bool) -> u32 {
        self.n += 1;
        let n = self.n;
        self.placeholders.insert(n, Placeholder { n, hint: truncate(hint), render, hard_break });
        self.parts.push_str(&format!("<x{n}/>"));
        n
    }

    /// Placeholder for literal bytes whose hint is the bytes themselves.
    pub fn literal(&mut self, raw: &str) -> u32 {
        self.placeholder(Render::literal(raw), raw, false)
    }

    pub fn open(&mut self, kind: PairKind, open: Render, close: Render, hint: &str) -> u32 {
        self.n += 1;
        let n = self.n;
        self.pairs.insert(n, Pair { n, kind, hint: truncate(hint), open, close, ref_label: None, close_original: None, quote: None });
        self.parts.push_str(&format!("<g{n}>"));
        n
    }

    pub fn close(&mut self, n: u32) {
        self.parts.push_str(&format!("</g{n}>"));
    }

    /// Adds raw source text: protects identifiers/paths/escapes and turns line breaks into spaces.
    pub fn source(&mut self, raw: &str, mode: EscapeMode) {
        let mut lines: Vec<&str> = Vec::new();
        let mut breaks = Vec::new();
        let mut last = 0;
        for m in SOFT_BREAK().all(raw) {
            lines.push(&raw[last..m.start]);
            breaks.push((m.text.to_string(), m.g(1).to_string(), m.g(2).to_string()));
            last = m.end;
        }
        lines.push(&raw[last..]);
        let is_fence: Vec<bool> = lines.iter().map(|l| FENCE_LINE().test(l)).collect();
        let has_fence = is_fence.iter().any(|f| *f);
        let mut fences: Vec<String> = Vec::new();
        let mut run: Option<u32> = None;
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                let (whole, eol, prefix) = &breaks[i - 1];
                if is_fence[i] || is_fence[i - 1] {
                    // Line breaks around fence lines are structure and must stay where they are.
                    if let Some(r) = run.take() {
                        self.close(r);
                    }
                    let x = self.placeholder(Render::literal(whole.as_str()), "line break", true);
                    fences.push(format!("x{x}"));
                } else {
                    if self.soft_break.is_none() {
                        self.soft_break = Some(SoftBreakInfo { eol: eol.clone(), prefix: prefix.clone() });
                    }
                    self.text(" ");
                }
            }
            if is_fence[i] {
                if let Some(r) = run.take() {
                    self.close(r);
                }
                let toks = self.fence_line(line, mode);
                fences.extend(toks);
            } else {
                // Anchor each text run between fence lines in a pair so moving text across fences is detectable.
                if has_fence && run.is_none() && !line.is_empty() {
                    let g = self.open(PairKind::Html, Render::literal(""), Render::literal(""), "text between directive lines");
                    run = Some(g);
                    fences.push(format!("g{g}"));
                }
                self.source_line(line, mode);
            }
        }
        if let Some(r) = run.take() {
            self.close(r);
        }
        if fences.len() > 1 {
            self.tag_groups.push(TagGroup { tokens: fences, contiguous: false });
        }
    }

    /// `:::name Title` keeps the fence and translates the title; `:::image alt-text="..." :::` translates only prose attributes.
    fn fence_line(&mut self, line: &str, mode: EscapeMode) -> Vec<String> {
        if !line.starts_with(":::") || ATTRIBUTE_ASSIGNMENT().test(line) {
            return self.attributed(line);
        }
        if let Some(m) = FENCE_TITLE().exec(line) {
            let (head, title) = (m.g(1), m.g(2));
            if has_letter(title) && !TRAILING_FENCE().test(title) {
                let x = self.literal(head);
                let g = self.open(PairKind::Html, Render::literal(""), Render::literal(""), "directive title");
                self.source_line(title, mode);
                self.close(g);
                return vec![format!("x{x}"), format!("g{g}")];
            }
        }
        vec![format!("x{}", self.literal(line))]
    }

    /// Protected token with translatable attribute values (Hugo shortcodes, Docs directives). Returns the tag ids.
    pub fn attributed(&mut self, raw: &str) -> Vec<String> {
        let values: Vec<_> = ATTR_VALUE().all(raw).into_iter().filter(|v| has_letter(v.g(4))).collect();
        if values.is_empty() {
            return vec![format!("x{}", self.literal(raw))];
        }
        let mut tokens = Vec::new();
        let mut pos = 0;
        for v in values {
            let start = v.start + v.g(1).len() + v.g(2).len() + 1;
            tokens.push(format!("x{}", self.literal(&raw[pos..start])));
            let g = self.open(PairKind::Html, Render::literal(""), Render::literal(""), &format!("{} attribute value", v.g(1)));
            self.pairs.get_mut(&g).unwrap().quote = v.g(3).chars().next();
            tokens.push(format!("g{g}"));
            self.source_line(v.g(4), EscapeMode::None);
            self.close(g);
            pos = start + v.g(4).len();
        }
        tokens.push(format!("x{}", self.literal(&raw[pos..])));
        self.tag_groups.push(TagGroup { tokens: tokens.clone(), contiguous: true });
        tokens
    }

    fn source_line(&mut self, raw: &str, mode: EscapeMode) {
        if raw.is_empty() {
            return;
        }
        #[derive(Clone, Copy, PartialEq)]
        enum K {
            Protect(&'static str),
            Escape,
            Entity,
        }
        let spans: Vec<(usize, usize, K)> = find_protected(raw, self.dnt).into_iter().map(|s| (s.start, s.end, K::Protect(s.rule))).collect();
        let mut extra: Vec<(usize, usize, K)> = Vec::new();
        if mode == EscapeMode::Markdown {
            for m in MD_ESCAPE().all(raw) {
                extra.push((m.start, m.end, K::Escape));
            }
        }
        if mode != EscapeMode::None {
            for m in ENTITY().all(raw) {
                extra.push((m.start, m.end, K::Entity));
            }
        }
        // Escapes inside a protected span stay part of that span.
        let mut all: Vec<(usize, usize, K)> = spans.clone();
        all.extend(extra.into_iter().filter(|e| !spans.iter().any(|s| e.0 >= s.0 && e.1 <= s.1)));
        all.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        let mut pos = 0;
        for (start, end, kind) in all {
            if start < pos {
                continue;
            }
            self.text(&raw[pos..start]);
            let bytes = &raw[start..end];
            if kind == K::Protect("template") && SHORTCODE_START().test(bytes) {
                self.attributed(bytes);
            } else {
                let hint = if kind == K::Escape { &bytes[1..] } else { bytes };
                self.placeholder(Render::literal(bytes), hint, false);
            }
            pos = end;
        }
        self.text(&raw[pos..]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaskToken {
    Text(String),
    /// Placeholder; a negative number is a self-closed pair tag (`<g3/>`), which is invalid.
    X(i64),
    Open(u32),
    Close(u32),
}

rx!(TAG, r"<(\/?)([gx])(\d+)\s*(\/?)>");

pub fn tokenize(masked: &str) -> Vec<MaskToken> {
    let mut out = Vec::new();
    let mut last = 0;
    for m in TAG().all(masked) {
        if m.start > last {
            out.push(MaskToken::Text(masked[last..m.start].to_string()));
        }
        let n: i64 = m.g(3).parse().unwrap_or(i64::MAX);
        let n32 = u32::try_from(n).unwrap_or(u32::MAX);
        if m.g(2) == "x" {
            out.push(MaskToken::X(n));
        } else if m.g(1) == "/" {
            out.push(MaskToken::Close(n32));
        } else if m.g(4) == "/" {
            out.push(MaskToken::X(-n));
        } else {
            out.push(MaskToken::Open(n32));
        }
        last = m.end;
    }
    if last < masked.len() {
        out.push(MaskToken::Text(masked[last..].to_string()));
    }
    out
}

/// Structural tag check: every placeholder once, every pair opened and closed once and properly nested.
pub fn check_tags(seg: &Segment, translated: &str) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let tokens = tokenize(translated);
    let mut seen_x: BTreeMap<u32, usize> = BTreeMap::new();
    let mut seen_open = std::collections::BTreeSet::new();
    let mut seen_close = std::collections::BTreeSet::new();
    let mut stack: Vec<u32> = Vec::new();
    for tok in &tokens {
        match tok {
            MaskToken::X(n) => {
                if *n < 0 || !seg.placeholders.contains_key(&(*n as u32)) {
                    errors.push(format!("unknown tag <x{}/>", n.unsigned_abs()));
                    continue;
                }
                *seen_x.entry(*n as u32).or_insert(0) += 1;
            }
            MaskToken::Open(n) => {
                if !seg.pairs.contains_key(n) {
                    errors.push(format!("unknown tag <g{n}>"));
                } else if seen_open.contains(n) {
                    errors.push(format!("<g{n}> used more than once"));
                }
                seen_open.insert(*n);
                stack.push(*n);
            }
            MaskToken::Close(n) => {
                if !seg.pairs.contains_key(n) {
                    errors.push(format!("unknown tag </g{n}>"));
                }
                if stack.last() != Some(n) {
                    errors.push(format!("</g{n}> closes out of order"));
                } else {
                    stack.pop();
                }
                seen_close.insert(*n);
            }
            MaskToken::Text(_) => {}
        }
    }
    for n in seg.placeholders.keys() {
        let c = seen_x.get(n).copied().unwrap_or(0);
        if c != 1 {
            errors.push(if c == 0 { format!("missing <x{n}/>") } else { format!("<x{n}/> used {c} times") });
        }
    }
    for n in seg.pairs.keys() {
        if !seen_open.contains(n) || !seen_close.contains(n) {
            errors.push(format!("pair <g{n}>…</g{n}> missing"));
        }
    }
    if !stack.is_empty() {
        errors.push(format!("unclosed tags: {}", stack.iter().map(|n| format!("<g{n}>")).collect::<Vec<_>>().join(", ")));
    }
    errors.extend(check_tag_groups(seg, &tokens));
    let mut unique = Vec::new();
    for e in errors {
        if !unique.contains(&e) {
            unique.push(e);
        }
    }
    unique
}

fn token_key(t: &MaskToken) -> String {
    match t {
        MaskToken::X(n) => format!("x{n}"),
        MaskToken::Open(n) => format!("g{n}"),
        MaskToken::Close(n) => format!("/g{n}"),
        MaskToken::Text(_) => String::new(),
    }
}

/// Order constraints for fence lines and attribute values; quotes must not leak into attribute values.
fn check_tag_groups(seg: &Segment, tokens: &[MaskToken]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut inside: Vec<u32> = Vec::new();
    for t in tokens {
        match t {
            MaskToken::Open(n) => inside.push(*n),
            MaskToken::Close(_) => {
                inside.pop();
            }
            MaskToken::Text(v) => {
                for n in &inside {
                    if let Some(q) = seg.pairs.get(n).and_then(|p| p.quote) {
                        if xml_unescape(v).contains(q) {
                            errors.push(format!("text inside <g{n}> must not contain {q}"));
                        }
                    }
                }
            }
            MaskToken::X(_) => {}
        }
    }
    let keys: Vec<String> = tokens.iter().map(token_key).collect();
    let index_of = |k: &str, from: usize| keys.iter().skip(from).position(|x| x == k).map(|p| p + from);
    for group in &seg.tag_groups {
        let positions: Vec<Option<usize>> = group.tokens.iter().map(|k| index_of(k, 0)).collect();
        let bad = positions.iter().enumerate().any(|(i, p)| p.is_none() || (i > 0 && p.unwrap() <= positions[i - 1].unwrap_or(usize::MAX)));
        if bad {
            let list: Vec<String> = group.tokens.iter().map(|k| format!("<{k}{}>", if k.starts_with('x') { "/" } else { "" })).collect();
            errors.push(format!("keep {} in their original order", list.join(" ")));
            continue;
        }
        if !group.contiguous {
            continue;
        }
        // Contiguous: nothing but the attribute text (inside <gN>…</gN>) may appear between the group's tags.
        let mut i = positions[0].unwrap();
        let mut ok = true;
        for k in &group.tokens {
            if keys.get(i).map(String::as_str) != Some(k.as_str()) {
                ok = false;
                break;
            }
            i = if k.starts_with('g') { index_of(&format!("/{k}"), i).map(|p| p + 1).unwrap_or(0) } else { i + 1 };
        }
        if !ok {
            let gs: Vec<String> = group.tokens.iter().filter(|k| k.starts_with('g')).map(|k| format!("<{k}>")).collect();
            errors.push(format!("do not move text into or out of the attribute values {}", gs.join(", ")));
        }
    }
    errors
}

pub fn plain_text(masked: &str) -> String {
    tokenize(masked)
        .into_iter()
        .filter_map(|t| match t {
            MaskToken::Text(v) => Some(xml_unescape(&v)),
            _ => None,
        })
        .collect()
}

const MD_SPECIAL: [char; 13] = ['\\', '`', '*', '_', '[', ']', '<', '>', '~', '|', '&', '#', '!'];

rx!(BARE_AMP, r"&(?!(?:[A-Za-z][A-Za-z0-9]{1,31}|#\d{1,7}|#[xX][0-9a-fA-F]{1,6});)");

/// Escaper for translated text in a given context. For Markdown only characters the translation introduced
/// (more occurrences than in the source) are escaped, so untouched literal characters keep their bytes.
pub struct Escaper {
    ctx: TextContext,
    chars: Vec<char>,
    quote: Option<char>,
}

impl Escaper {
    pub fn new(ctx: TextContext, translated_text: &str, source_text: &str, quote: Option<char>) -> Escaper {
        let chars = match ctx {
            TextContext::Markdown | TextContext::Cell => MD_SPECIAL
                .iter()
                .copied()
                .filter(|ch| (ctx == TextContext::Cell && *ch == '|') || count(translated_text, *ch) > count(source_text, *ch))
                .collect(),
            _ => Vec::new(),
        };
        Escaper { ctx, chars, quote }
    }

    pub fn escape(&self, s: &str) -> String {
        match self.ctx {
            TextContext::Markdown | TextContext::Cell => {
                let mut out = s.to_string();
                for ch in &self.chars {
                    out = out.replace(*ch, &format!("\\{ch}"));
                }
                out
            }
            TextContext::Html => BARE_AMP().replace_all_str(s, "&amp;").replace('<', "&lt;"),
            TextContext::Attr => {
                let mut out = BARE_AMP().replace_all_str(s, "&amp;").replace('<', "&lt;");
                if self.quote == Some('"') {
                    out = out.replace('"', "&quot;");
                }
                if self.quote == Some('\'') {
                    out = out.replace('\'', "&#39;");
                }
                out
            }
            TextContext::MdTitle => match self.quote {
                Some('(') => s.replace('(', "\\(").replace(')', "\\)"),
                Some(q) => s.replace(q, &format!("\\{q}")),
                None => s.to_string(),
            },
            TextContext::Comment | TextContext::Yaml => s.to_string(),
        }
    }
}

fn count(s: &str, ch: char) -> usize {
    s.chars().filter(|c| *c == ch).count()
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub s: String,
    pub atom: bool,
    pub hard_break: bool,
}

/// Converts a translated masked string into output pieces (text is unescaped, then escaped for its context).
pub fn render_pieces(seg: &Segment, translated: &str, tm: &TMap, store: &SegmentStore) -> Vec<Piece> {
    let tokens = normalize_emphasis_whitespace(seg, tokenize(translated));
    let texts: Vec<String> = tokens
        .iter()
        .filter_map(|t| match t {
            MaskToken::Text(v) => Some(xml_unescape(v)),
            _ => None,
        })
        .collect();
    let escaper = Escaper::new(seg.text_context, &texts.concat(), &seg.source_text, seg.quote);
    let mut pieces: Vec<Piece> = Vec::new();
    let mut ti = 0;
    let mut open_at: BTreeMap<u32, usize> = BTreeMap::new();
    for tok in &tokens {
        match tok {
            MaskToken::Text(_) => {
                pieces.push(Piece { s: escaper.escape(&texts[ti]), atom: false, hard_break: false });
                ti += 1;
            }
            MaskToken::X(n) => {
                if let Some(p) = u32::try_from(*n).ok().and_then(|n| seg.placeholders.get(&n)) {
                    pieces.push(Piece { s: crate::markdown::render::render_value(&p.render, tm, store), atom: true, hard_break: p.hard_break });
                }
            }
            MaskToken::Open(n) => {
                if let Some(pair) = seg.pairs.get(n) {
                    open_at.insert(*n, pieces.len());
                    pieces.push(Piece { s: crate::markdown::render::render_value(&pair.open, tm, store), atom: true, hard_break: false });
                }
            }
            MaskToken::Close(n) => {
                if let Some(pair) = seg.pairs.get(n) {
                    let from = open_at.get(n).map(|i| i + 1).unwrap_or(pieces.len());
                    let inner: String = pieces[from.min(pieces.len())..].iter().map(|p| p.s.as_str()).collect();
                    let unchanged_ref = pair.ref_label.as_deref() == Some(inner.as_str());
                    let s = if unchanged_ref {
                        pair.close_original.clone().unwrap_or_default()
                    } else {
                        crate::markdown::render::render_value(&pair.close, tm, store)
                    };
                    pieces.push(Piece { s, atom: true, hard_break: false });
                }
            }
        }
    }
    pieces
}

fn leading_space(s: &str) -> &str {
    jsstr::leading_ws(s)
}

fn trailing_space(s: &str) -> &str {
    jsstr::trailing_ws(s)
}

/// Emphasis delimiters must hug their content: "<g1> text</g1>" becomes " <g1>text</g1>".
fn normalize_emphasis_whitespace(seg: &Segment, tokens: Vec<MaskToken>) -> Vec<MaskToken> {
    let mut out = tokens;
    let is_em = |n: u32| seg.pairs.get(&n).is_some_and(|p| p.kind.is_emphasis());
    let mut i = 0;
    while i < out.len() {
        match out[i].clone() {
            MaskToken::Open(n) if is_em(n) => {
                if let Some(MaskToken::Text(next)) = out.get(i + 1).cloned() {
                    let ws = leading_space(&next).to_string();
                    if !ws.is_empty() {
                        out[i + 1] = MaskToken::Text(next[ws.len()..].to_string());
                        if i > 0 {
                            if let MaskToken::Text(prev) = &mut out[i - 1] {
                                prev.push_str(&ws);
                                i += 1;
                                continue;
                            }
                        }
                        out.insert(i, MaskToken::Text(ws));
                        i += 1;
                    }
                }
            }
            MaskToken::Close(n) if is_em(n) => {
                if i > 0 {
                    if let MaskToken::Text(prev) = out[i - 1].clone() {
                        let ws = trailing_space(&prev).to_string();
                        if !ws.is_empty() {
                            out[i - 1] = MaskToken::Text(prev[..prev.len() - ws.len()].to_string());
                            if let Some(MaskToken::Text(next)) = out.get_mut(i + 1) {
                                *next = format!("{ws}{next}");
                            } else {
                                out.insert(i + 1, MaskToken::Text(ws));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    out.into_iter().filter(|t| !matches!(t, MaskToken::Text(v) if v.is_empty())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_protected_spans_and_escapes() {
        let dnt = vec![];
        let mut mb = MaskBuilder::new(&dnt);
        mb.source("Use \\*stars\\* and &copy; in test.jpeg", EscapeMode::Markdown);
        assert_eq!(mb.masked(), "Use <x1/>stars<x2/> and <x3/> in <x4/>");
        assert_eq!(mb.placeholders[&1].hint, "*");
    }

    #[test]
    fn soft_breaks_become_spaces() {
        let dnt = vec![];
        let mut mb = MaskBuilder::new(&dnt);
        mb.source("first line\n> second line", EscapeMode::Markdown);
        assert_eq!(mb.masked(), "first line second line");
        let sb = mb.soft_break.unwrap();
        assert_eq!((sb.eol.as_str(), sb.prefix.as_str()), ("\n", "> "));
    }

    #[test]
    fn fence_lines_keep_breaks() {
        let dnt = vec![];
        let mut mb = MaskBuilder::new(&dnt);
        mb.source(":::tip Pro tip\nUse the cache.\n:::", EscapeMode::Markdown);
        assert_eq!(mb.masked(), "<x1/><g2>Pro tip</g2><x3/><g4>Use the cache.</g4><x5/><x6/>");
        assert_eq!(mb.tag_groups.len(), 1);
    }

    #[test]
    fn tokenize_and_check() {
        let t = tokenize("a <g1>b</g1> <x2/> <g3/>");
        assert_eq!(t[1], MaskToken::Open(1));
        assert_eq!(t[5], MaskToken::X(2));
        assert_eq!(t[7], MaskToken::X(-3));
    }
}
