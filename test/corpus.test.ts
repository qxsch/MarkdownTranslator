import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { initCodeParsers } from '../src/code/parsers.js';
import { extract } from '../src/markdown/extract.js';
import { assembleDocument } from '../src/assemble.js';
import { validateSegment } from '../src/translate/validate.js';
import { LANG, pseudoMap } from './helpers.js';

await initCodeParsers();

// The feature-evaluation corpus must be sound before it is used to measure anything.
const dir = new URL('../eval/features/', import.meta.url);
const files = readdirSync(dir).filter((f) => /\.mdx?$/.test(f)).sort();
const expectations: Record<string, { keep: string[]; translate: string[] }> = JSON.parse(readFileSync(new URL('expect.json', dir), 'utf8'));

describe('feature evaluation corpus', () => {
  it('has at least 50 documents, each with expectations', () => {
    expect(files.length).toBeGreaterThanOrEqual(50);
    expect(Object.keys(expectations).sort()).toEqual(files);
  });

  it.each(files)('%s', (file) => {
    const src = readFileSync(new URL(file, dir), 'utf8');
    const e = expectations[file];
    for (const s of [...e.keep, ...e.translate]) expect(src, `expectation not in source: ${JSON.stringify(s)}`).toContain(s);

    const ex = extract(src, [], { parse: { mdx: file.endsWith('.mdx') } });
    expect(assembleDocument(ex, new Map(), { wrap: true, preserveAnchors: true }).text).toBe(src);

    const tm = pseudoMap(ex);
    const failures = ex.segments.filter((s) => !s.passive && validateSegment(s, tm.get(s.id)!, ex, LANG).length).map((s) => s.masked);
    expect(failures).toEqual([]);
    const out = assembleDocument(ex, tm, { wrap: true, preserveAnchors: true });
    expect(out.reverted).toEqual([]);
    // With every feature on, each phrase that must be translated lies inside a translatable segment.
    for (const s of e.translate) expect(out.text, `not extracted for translation: ${JSON.stringify(s)}`).not.toContain(s);
  });
});
