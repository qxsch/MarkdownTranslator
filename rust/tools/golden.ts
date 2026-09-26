/**
 * Golden files for the Rust port: every fixture is extracted, pseudo-translated (test/helpers.ts `pseudo`) and
 * assembled by the TypeScript implementation, with the default switches and with each switch that changes
 * extraction or assembly flipped. The Rust test suite (rust/tests/golden.rs) must reproduce them exactly.
 *
 *   npx tsx rust/tools/golden.ts            # rewrite rust/tests/golden
 *   npx tsx rust/tools/golden.ts --check    # compare the Rust binary against the TypeScript implementation live
 *   npx tsx rust/tools/golden.ts --check --dir <docs folder>   # any Markdown folder (real-world parity)
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { extract } from '../../src/markdown/extract.ts';
import { assembleDocument } from '../../src/assemble.ts';
import { initCodeParsers } from '../../src/code/parsers.ts';
import { loadGlossary } from '../../src/config.ts';
import { pseudoMap } from '../../test/helpers.ts';
import type { Extraction } from '../../src/markdown/types.ts';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const goldenDir = join(root, 'rust', 'tests', 'golden');
const args = process.argv.slice(2);
const check = args.includes('--check');
const dirArg = args.includes('--dir') ? args[args.indexOf('--dir') + 1] : undefined;

await initCodeParsers();
const dnt = loadGlossary(join(root, 'config', 'glossary.json')).doNotTranslate;

export interface Variant {
  mdx: boolean;
  mathSingleDollar: boolean;
  docstrings: boolean;
  codeComments: boolean;
  frontMatter: boolean;
  preserveAnchors: boolean;
}

const byteOffset = (s: string) => {
  const table: number[] = [];
  let b = 0;
  for (let i = 0; i < s.length; i++) {
    table[i] = b;
    const c = s.codePointAt(i)!;
    if (c > 0xffff) {
      table[i + 1] = b;
      b += 4;
      i++;
    } else b += c < 0x80 ? 1 : c < 0x800 ? 2 : 3;
  }
  table[s.length] = b;
  return (o: number) => table[o];
};

function dumpExtraction(ex: Extraction) {
  const at = byteOffset(ex.source);
  return {
    segments: ex.segments.map((s) => {
      const tags: Record<string, string> = {};
      for (const p of s.placeholders.values()) tags[`x${p.n}`] = `${p.hint}${p.hardBreak ? ' [break]' : ''}`;
      for (const p of s.pairs.values()) tags[`g${p.n}`] = `${p.kind}: ${p.hint}${p.quote ? ` [quote ${p.quote}]` : ''}${p.refLabel !== undefined ? ` [ref ${p.refLabel}]` : ''}`;
      return {
        id: s.id,
        kind: s.kind,
        textContext: s.textContext,
        note: s.note,
        structure: s.structure ?? null,
        masked: s.masked,
        original: s.original,
        sourceText: s.sourceText,
        passive: !!s.passive,
        embedded: !!s.embedded,
        dependents: s.dependents ?? [],
        tags,
        tagGroups: (s.tagGroups ?? []).map((g) => `${g.contiguous ? 'contiguous' : 'ordered'}: ${g.tokens.join(' ')}`),
        wrap: s.wrap ? { width: s.wrap.width, firstColumn: s.wrap.firstColumn, prefix: s.wrap.prefix, eol: s.wrap.eol } : null,
        forbidden: s.forbidden ?? [],
        quote: s.quote || null,
        inlineSignature: s.inlineSignature ?? null,
      };
    }),
    replacements: ex.replacements.map((r) => [at(r.start), at(r.end), r.segmentIds[0]]),
    notes: ex.notes,
    definitionsText: ex.definitionsText,
    frontmatterFormality: ex.frontmatterFormality ?? null,
  };
}

export function run(src: string, v: Variant) {
  let ex: Extraction;
  try {
    ex = extract(src, dnt, { parse: { mdx: v.mdx, mathSingleDollar: v.mathSingleDollar }, docstrings: v.docstrings, codeComments: v.codeComments, frontMatter: v.frontMatter });
  } catch (e) {
    return { error: `extract: ${(e as Error).message}` };
  }
  const dump = dumpExtraction(ex);
  try {
    const out = assembleDocument(ex, pseudoMap(ex), { wrap: true, preserveAnchors: v.preserveAnchors });
    return { ...dump, output: out.text, reverted: out.reverted, anchors: out.anchors };
  } catch (e) {
    return { ...dump, error: `assemble: ${(e as Error).message}` };
  }
}

// rust/tests/fixtures holds parity-only inputs (Rust regressions) that the TypeScript tests don't need.
const FIXTURE_DIRS = ['eval/features', 'eval/corpus', 'test/fixtures', 'rust/tests/fixtures'];

function fixtures(dirs: string[]): string[] {
  const walk = (d: string): string[] =>
    readdirSync(d).flatMap((f) => {
      const p = join(d, f);
      if (statSync(p).isDirectory()) return f === 'node_modules' || f.startsWith('.') ? [] : walk(p);
      return /\.mdx?$/i.test(f) ? [p] : [];
    });
  return dirs.flatMap(walk).sort();
}

const FLIPS: (keyof Variant)[] = ['mdx', 'mathSingleDollar', 'docstrings', 'codeComments', 'frontMatter', 'preserveAnchors'];
const defaults = (file: string): Variant => ({ mdx: /\.mdx$/i.test(file), mathSingleDollar: false, docstrings: true, codeComments: true, frontMatter: true, preserveAnchors: true });
const variantName = (v: Variant, d: Variant) => FLIPS.filter((k) => v[k] !== d[k]).map((k) => `${k}=${v[k]}`).join(',') || 'default';

function cases(files: string[]) {
  const out: { file: string; variant: Variant; name: string }[] = [];
  for (const file of files) {
    const src = readFileSync(file, 'utf8');
    const d = defaults(file);
    const base = JSON.stringify(run(src, d));
    out.push({ file, variant: d, name: 'default' });
    for (const k of FLIPS) {
      const v = { ...d, [k]: !d[k] };
      if (JSON.stringify(run(src, v)) !== base) out.push({ file, variant: v, name: variantName(v, d) });
    }
  }
  return out;
}

const slugOf = (file: string, name: string) => `${relative(root, file).replace(/[\\/]/g, '__')}${name === 'default' ? '' : `@${name}`}.json`;

if (!check) {
  rmSync(goldenDir, { recursive: true, force: true });
  mkdirSync(goldenDir, { recursive: true });
  const all = cases(fixtures(FIXTURE_DIRS.map((d) => join(root, d))));
  const extractionOf = (r: Record<string, unknown>) => JSON.stringify({ ...r, output: undefined, reverted: undefined, anchors: undefined, error: undefined });
  const defaultsByFile = new Map<string, string>();
  for (const c of all) {
    const res = run(readFileSync(c.file, 'utf8'), c.variant) as Record<string, unknown>;
    const file = relative(root, c.file).replace(/\\/g, '/');
    let body: Record<string, unknown> = res;
    if (c.name === 'default') defaultsByFile.set(file, extractionOf(res));
    else if (defaultsByFile.get(file) === extractionOf(res) && !('error' in res && String(res.error).startsWith('extract'))) {
      // Only assembly differs from the default variant: keep the output, not a second copy of the extraction.
      body = { sameExtractionAsDefault: true, output: res.output, reverted: res.reverted, anchors: res.anchors, ...('error' in res ? { error: res.error } : {}) };
    }
    writeFileSync(join(goldenDir, slugOf(c.file, c.name)), JSON.stringify({ file, options: c.variant, ...body }, null, 1) + '\n');
  }
  console.log(`wrote ${all.length} golden files to ${relative(root, goldenDir)}`);
} else {
  // Live comparison against the Rust binary (`-dumpGolden` prints the same JSON).
  const exe = process.env.MDT_RUST_BIN ?? join(root, 'rust', 'target', process.env.RUST_PROFILE ?? 'release', process.platform === 'win32' ? 'mdtranslate.exe' : 'mdtranslate');
  if (!existsSync(exe)) throw new Error(`build the binary first: cargo build --release (looked for ${exe}; or set MDT_RUST_BIN)`);
  const files = fixtures(dirArg ? [resolve(dirArg)] : FIXTURE_DIRS.map((d) => join(root, d)));
  // Error texts differ between the implementations; only the failing stage has to match.
  const normalize = (v: Record<string, unknown>) => {
    const out: Record<string, unknown> = { ...v };
    if (typeof out.error === 'string') out.error = out.error.split(':')[0];
    if (Array.isArray(out.notes)) out.notes = (out.notes as string[]).map((n) => n.replace(/\(YAML error: .*$/, '(YAML error)'));
    return out;
  };
  const firstDiff = (a: unknown, b: unknown, path = ''): string | undefined => {
    if (JSON.stringify(a) === JSON.stringify(b)) return undefined;
    if (a && b && typeof a === 'object' && typeof b === 'object') {
      for (const k of new Set([...Object.keys(a as object), ...Object.keys(b as object)])) {
        const d = firstDiff((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k], `${path}.${k}`);
        if (d) return d;
      }
    }
    const show = (x: unknown) => JSON.stringify(x)?.slice(0, 160);
    return `${path}: TypeScript ${show(a)} / Rust ${show(b)}`;
  };
  let same = 0;
  const diffs: string[] = [];
  for (const file of files) {
    const v = defaults(file);
    const ts = normalize(run(readFileSync(file, 'utf8'), v) as Record<string, unknown>);
    let rs: Record<string, unknown>;
    try {
      rs = normalize(JSON.parse(execFileSync(exe, ['-sourceFile', file, '-targetFile', '-', '-dumpGolden', ...(v.mdx ? ['-mdx'] : ['-no-mdx'])], { encoding: 'utf8', maxBuffer: 1 << 28 })));
    } catch (e) {
      rs = { error: `binary: ${(e as Error).message.split('\n')[0]}` };
    }
    const d = firstDiff(ts, rs);
    if (!d) same++;
    else diffs.push(`${relative(process.cwd(), file)}\n      ${d}`);
  }
  console.log(`identical: ${same}/${files.length}`);
  if (diffs.length) console.log(`different:\n  ${diffs.join('\n  ')}`);
  process.exitCode = diffs.length ? 1 : 0;
}
