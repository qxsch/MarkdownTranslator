//! Splices translations into the source and proves that the Markdown structure is unchanged; blocks that
//! would change the structure are reverted to the source text. Port of `assemble.ts`.

use crate::markdown::anchors::preserve_anchors;
use crate::markdown::document::{apply_translations, compare_skeletons, skeleton, SkeletonItem};
use crate::markdown::render::render_segment;
use crate::types::{Extraction, RenderOptions, TMap};

pub struct AssembleOptions {
    pub wrap: bool,
    pub preserve_anchors: bool,
}

pub struct Assembled {
    pub text: String,
    pub reverted: Vec<String>,
    pub anchors: Vec<String>,
}

pub fn assemble_document(ex: &Extraction, tm: &TMap, o: &AssembleOptions) -> Result<Assembled, String> {
    let source = skeleton(&ex.source, ex.parse_options)?;
    let mut work = tm.clone();
    let mut reverted: Vec<String> = Vec::new();
    let mut opts = RenderOptions::wrapped(o.wrap);
    let render = |work: &TMap, opts: RenderOptions| -> Result<(String, Vec<SkeletonItem>), String> {
        let text = apply_translations(ex, work, opts);
        let sk = skeleton(&text, ex.parse_options)?;
        Ok((text, sk))
    };
    let (mut text, mut sk) = render(&work, opts)?;
    let mut diff = compare_skeletons(&source, &sk);
    if diff.is_some() && opts.wrap {
        opts.wrap = false;
        (text, sk) = render(&work, opts)?;
        diff = compare_skeletons(&source, &sk);
    }
    let mut guard = 0;
    while let Some(d) = diff {
        if guard >= 100 {
            break;
        }
        guard += 1;
        let item = sk.get(d).or(sk.last());
        let prev = if d > 0 { sk.get(d - 1) } else { None };
        let culprits = replacements_near(ex, &work, opts, item, prev);
        if culprits.is_empty() {
            break;
        }
        for id in culprits {
            work.remove(&id);
            if let Some(seg) = ex.get(&id) {
                for dep in &seg.dependents {
                    work.remove(dep);
                }
            }
            reverted.push(id);
        }
        (text, sk) = render(&work, opts)?;
        diff = compare_skeletons(&source, &sk);
    }
    if let Some(d) = diff {
        let near = sk.get(d).map(|s| s.sig.chars().take(120).collect::<String>()).unwrap_or_else(|| "end of document".into());
        return Err(format!("translated document failed the structure check near: {near}"));
    }
    let mut anchors = Vec::new();
    if o.preserve_anchors {
        let (t, added) = preserve_anchors(&ex.source, &text, ex.parse_options)?;
        text = t;
        anchors = added;
    }
    Ok(Assembled { text: format!("{}{}", ex.bom, text), reverted, anchors })
}

/// Translated replacements overlapping the mismatching node (or, failing that, its predecessor).
fn replacements_near(ex: &Extraction, tm: &TMap, opts: RenderOptions, item: Option<&SkeletonItem>, prev: Option<&SkeletonItem>) -> Vec<String> {
    let mut ranges: Vec<(usize, usize, Vec<String>)> = Vec::new();
    let mut pos = 0;
    let mut out_pos = 0;
    for r in &ex.replacements {
        out_pos += r.start - pos;
        let Some(seg) = ex.get(&r.seg) else { continue };
        let rendered = render_segment(seg, tm, &ex.store, opts);
        let touched = tm.contains_key(&seg.id) || seg.dependents.iter().any(|d| tm.contains_key(d));
        if rendered != ex.source[r.start..r.end] && touched {
            ranges.push((out_pos, out_pos + rendered.len(), vec![seg.id.clone()]));
        }
        out_pos += rendered.len();
        pos = r.end;
    }
    let hit = |x: Option<&SkeletonItem>| -> Vec<String> {
        match x {
            Some(x) => ranges.iter().filter(|(s, e, _)| *s <= x.end && *e >= x.start).flat_map(|(_, _, ids)| ids.clone()).collect(),
            None => Vec::new(),
        }
    };
    let a = hit(item);
    if a.is_empty() {
        hit(prev)
    } else {
        a
    }
}
