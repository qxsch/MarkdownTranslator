//! Translatable text and attributes inside HTML blocks. Port of `markdown/html.ts`.

use super::context::{ExtractContext, SegmentOptions};
use super::html_attrs::{has_no_translate_marker, AttrSet, INLINE_ELEMENTS, NO_TRANSLATE_ELEMENTS, TRANSLATABLE_ATTRS, VOID_ELEMENTS};
use super::html_parser::{parse_fragment, Fragment, HKind};
use super::line_map::{column_u16, line_start_of, max_line_width, LineMap};
use crate::jsstr;
use crate::mask::masking::{EscapeMode, MaskBuilder};
use crate::types::{PairKind, Render, SegmentKind, TextContext, WrapSpec};

enum Map {
    Offset(usize),
    Lines(LineMap),
}

impl Map {
    fn map(&self, v: usize) -> usize {
        match self {
            Map::Offset(s) => v + s,
            Map::Lines(m) => m.map(v),
        }
    }
}

struct Html<'f> {
    frag: &'f Fragment,
    map: Map,
    value: &'f str,
}

fn is_no_translate(tag: &str, start_tag: &str) -> bool {
    NO_TRANSLATE_ELEMENTS.contains(&tag) || has_no_translate_marker(start_tag)
}

impl Html<'_> {
    fn slice<'s>(&self, src: &'s str, vs: usize, ve: usize) -> &'s str {
        &src[self.map.map(vs)..self.map.map(ve)]
    }

    fn is_inline(&self, id: usize) -> bool {
        let n = self.frag.node(id);
        match &n.kind {
            HKind::Text | HKind::Comment => true,
            HKind::Element { tag, .. } => INLINE_ELEMENTS.contains(&tag.as_str()) && n.children.iter().all(|c| self.is_inline(*c)),
        }
    }
}

pub fn process_html_block(ctx: &mut ExtractContext, start: usize, end: usize, value: &str) -> Result<(), String> {
    let src = ctx.source;
    let map = if value == &src[start..end] {
        Map::Offset(start)
    } else {
        match LineMap::build(src, line_start_of(src, start), value) {
            Some(m) => Map::Lines(m),
            None => {
                ctx.notes.push(format!("HTML block at offset {start} left untranslated (could not map container prefixes)"));
                return Ok(());
            }
        }
    };
    let frag = parse_fragment(value);
    let h = Html { frag: &frag, map, value };
    walk(ctx, &h, &frag.nodes[0].children.clone())
}

fn block_attributes(ctx: &mut ExtractContext, h: &Html, id: usize) {
    let src = ctx.source;
    let n = h.frag.node(id);
    let Some((ss, se)) = n.start_tag else { return };
    if !n.attrs().iter().any(|a| TRANSLATABLE_ATTRS.contains(&a.as_str())) {
        return;
    }
    let tag = h.slice(src, ss, se);
    let (render, ids) = ctx.tag_with_attributes(tag, &format!("HTML <{}>", n.tag().unwrap_or("")), AttrSet::Html);
    if ids.is_empty() {
        return;
    }
    let mut mb = ctx.builder();
    mb.placeholder(render, tag, false);
    let mut o = SegmentOptions::new(SegmentKind::Html, TextContext::Html, tag, "HTML tag");
    o.dependents = ids;
    o.force = true;
    if let Some(seg) = ctx.segment(mb, o) {
        ctx.replace(h.map.map(ss), h.map.map(se), seg);
    }
}

fn emit(ctx: &mut ExtractContext, h: &Html, id: usize, mb: &mut MaskBuilder, deps: &mut Vec<String>, clamp_start: usize, clamp_end: usize) -> Result<(), String> {
    let src = ctx.source;
    let n = h.frag.node(id);
    let (ls, le) = n.loc.ok_or("node without location")?;
    let vs = ls.max(clamp_start);
    let ve = le.min(clamp_end);
    let tag_name = match &n.kind {
        HKind::Text => {
            if ve > vs {
                mb.source(h.slice(src, vs, ve), EscapeMode::Html);
            }
            return Ok(());
        }
        HKind::Comment => {
            let raw = h.slice(src, ls, le);
            mb.placeholder(Render::literal(raw), raw, false);
            return Ok(());
        }
        HKind::Element { tag, .. } => tag.clone(),
    };
    let (ss, se) = n.start_tag.ok_or("element without start tag location")?;
    let tag = h.slice(src, ss, se);
    let void = VOID_ELEMENTS.contains(&tag_name.as_str());
    match n.end_tag {
        Some((es, ee)) if !void && !is_no_translate(&tag_name, tag) => {
            let (render, ids) = ctx.tag_with_attributes(tag, &format!("inline HTML <{tag_name}>"), AttrSet::Html);
            deps.extend(ids);
            let pair = mb.open(PairKind::Html, render, Render::literal(h.slice(src, es, ee)), &format!("<{tag_name}>"));
            for c in n.children.clone() {
                emit(ctx, h, c, mb, deps, clamp_start, clamp_end)?;
            }
            mb.close(pair);
        }
        end_tag => {
            if void && end_tag.is_none() {
                let (render, ids) = ctx.tag_with_attributes(tag, &format!("inline HTML <{tag_name}>"), AttrSet::Html);
                deps.extend(ids);
                mb.placeholder(render, tag, false);
            } else {
                let raw = h.slice(src, ls, le);
                mb.placeholder(Render::literal(raw), raw, false);
            }
        }
    }
    Ok(())
}

fn flush_run(ctx: &mut ExtractContext, h: &Html, run: &[usize]) -> Result<(), String> {
    if run.is_empty() {
        return Ok(());
    }
    let locs: Vec<Option<(usize, usize)>> = run.iter().map(|id| h.frag.node(*id).loc).collect();
    if locs.iter().any(|l| l.is_none()) || locs.windows(2).any(|w| w[1].unwrap().0 < w[0].unwrap().1) {
        return Ok(());
    }
    let mut vs = locs[0].unwrap().0;
    let mut ve = locs[locs.len() - 1].unwrap().1;
    if h.frag.node(run[0]).kind == HKind::Text {
        vs += jsstr::leading_ws(&h.value[vs..ve]).len();
    }
    if h.frag.node(run[run.len() - 1]).kind == HKind::Text {
        ve -= jsstr::trailing_ws(&h.value[vs..ve]).len();
    }
    if ve <= vs {
        return Ok(());
    }
    let src = ctx.source;
    let mut mb = ctx.builder();
    let mut deps = Vec::new();
    for id in run {
        emit(ctx, h, *id, &mut mb, &mut deps, vs, ve)?;
    }
    let from = h.map.map(vs);
    let to = h.map.map(ve);
    let wrap = mb.soft_break.as_ref().map(|sb| WrapSpec {
        width: max_line_width(src, from, to).max(40),
        first_column: column_u16(src, from),
        prefix: sb.prefix.clone(),
        eol: sb.eol.clone(),
    });
    let mut o = SegmentOptions::new(SegmentKind::Html, TextContext::Html, &src[from..to], "HTML text");
    o.wrap = wrap;
    o.dependents = deps;
    o.force = true;
    if let Some(seg) = ctx.segment(mb, o) {
        ctx.replace(from, to, seg);
    }
    Ok(())
}

fn walk(ctx: &mut ExtractContext, h: &Html, nodes: &[usize]) -> Result<(), String> {
    let src = ctx.source;
    let mut run: Vec<usize> = Vec::new();
    for &id in nodes {
        if h.is_inline(id) {
            run.push(id);
            continue;
        }
        flush_run(ctx, h, &run)?;
        run.clear();
        let n = h.frag.node(id);
        let HKind::Element { tag, .. } = &n.kind else { continue };
        if let Some((ss, se)) = n.start_tag {
            if is_no_translate(tag, h.slice(src, ss, se)) {
                continue;
            }
            block_attributes(ctx, h, id);
        }
        let children = n.children.clone();
        walk(ctx, h, &children)?;
    }
    flush_run(ctx, h, &run)
}
