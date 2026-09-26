/**
 * Measures what each feature switch brings. Every document is translated with the default configuration and with
 * exactly one switch flipped; both outputs are checked deterministically (expectations in eval/features/expect.json,
 * code bytes, structure, links, English left behind) and judged blind (position-swapped, two judges, MQM style).
 *
 *   npx tsx eval/features.ts
 *   EVAL_LANGS=de EVAL_DOCS=faq.md,math-prices.md EVAL_FEATURES=review,mathSingleDollar npx tsx eval/features.ts
 *   EVAL_OUT=eval/results/features-<stamp> npx tsx eval/features.ts   # reuse translations of an earlier run
 *
 * Unchanged segments are reused from the baseline (shared segment cache), so differences come from the feature, not
 * from sampling noise. Review off, NMT fallback off and anchors off are derived exactly from the baseline run.
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

if (existsSync('.env')) process.loadEnvFile('.env');
delete process.env.MDT_CACHE_DIR;

const { loadConfig } = await import('../src/config.js');
const { MarkdownTranslator } = await import('../src/pipeline.js');
const { extract } = await import('../src/markdown/extract.js');
const { assembleDocument } = await import('../src/assemble.js');
const { translateDocument } = await import('../src/translate/engine.js');
const { PROMPT_VERSION } = await import('../src/translate/prompts.js');
const { TranslationCache } = await import('../src/translate/cache.js');
const { parseMarkdown } = await import('../src/markdown/parse.js');
const { skeleton, compareSkeletons } = await import('../src/markdown/document.js');
const { codeFingerprint } = await import('../src/code/parsers.js');
const { ChatClient } = await import('../src/azure/openai.js');
const { Semaphore } = await import('../src/azure/http.js');
const { toString } = await import('mdast-util-to-string');
const { createJudges } = await import('./judges.js');
type Judge = import('./judges.js').Judge;
const { default: GithubSlugger } = await import('github-slugger');
type Outcome = import('../src/translate/engine.js').SegmentOutcome;
type Extraction = import('../src/markdown/types.js').Extraction;
type ParseOptions = import('../src/markdown/parse.js').ParseOptions;
type LanguageConfig = import('../src/config.js').LanguageConfig;

const cfg = loadConfig();
const t = await MarkdownTranslator.create({ ...cfg, cacheDir: undefined });
const translateLimiter = new Semaphore(cfg.maxConcurrency);

const FEATURES = ['review', 'structuralContext', 'nmtFallback', 'preserveAnchors', 'codeComments', 'docstrings', 'frontMatter', 'mdx', 'mathSingleDollar'] as const;
type Feature = (typeof FEATURES)[number];
type Settings = Record<Feature, boolean>;

const here = dirname(fileURLToPath(import.meta.url));
const corpusDir = join(here, 'features');
const LANGS = (process.env.EVAL_LANGS ?? 'de,fr').split(',');
const DOCS = (process.env.EVAL_DOCS?.split(',') ?? readdirSync(corpusDir).filter((f) => /\.mdx?$/.test(f))).sort();
const SELECTED = (process.env.EVAL_FEATURES?.split(',') as Feature[] | undefined) ?? [...FEATURES];
const JUDGES = (process.env.EVAL_JUDGES ?? 'translate-alt,translate').split(',');
const OUT = process.env.EVAL_OUT ?? join(here, 'results', `features-${new Date().toISOString().replace(/[:.]/g, '-')}`);
const expectations: Record<string, { keep: string[]; translate: string[] }> = JSON.parse(readFileSync(join(corpusDir, 'expect.json'), 'utf8'));
const WEIGHT = { minor: 1, major: 5, critical: 10 } as const;
// Derived exactly from the baseline instead of translating again (when the baseline has the feature on).
const DERIVED: Feature[] = ['review', 'nmtFallback', 'preserveAnchors'];
const NOT_JUDGED: Feature[] = ['preserveAnchors'];

const defaults = (doc: string): Settings => ({
  review: cfg.review, structuralContext: cfg.structuralContext, nmtFallback: cfg.nmtFallback, preserveAnchors: cfg.preserveAnchors,
  codeComments: cfg.codeComments, docstrings: cfg.docstrings, frontMatter: cfg.frontMatter, mdx: /\.mdx$/i.test(doc), mathSingleDollar: cfg.mathSingleDollar,
});
const naturalParse = (doc: string): ParseOptions => ({ mdx: /\.mdx$/i.test(doc) });
const comparisonOf = (f: Feature, doc: string) => (f === 'mdx' ? `mdx (${/\.mdx$/i.test(doc) ? '.mdx' : '.md'} files)` : f);

interface Run {
  text: string;
  error?: string;
  reverted: number;
  keptSource: number;
  nmt: number;
  reviewEdits: number;
  seconds: number;
  /** Model tokens (prompt + completion) spent by this run; cache-reused segments cost nothing. */
  tokens: { translate: number; review: number };
  /** Output taken from the baseline because the switch did not change the extraction. */
  reused?: boolean;
  ex?: Extraction;
  tm?: Map<string, string>;
  outcomes?: Map<string, Outcome>;
}

interface Job {
  doc: string;
  lang: LanguageConfig;
  src: string;
  analysis: unknown;
  formality: 'formal' | 'informal';
  cache: InstanceType<typeof TranslationCache>;
  base?: Run;
  variants: Partial<Record<Feature, Run>>;
}

// ------------------------------------------------------------------ translate

const extractFor = (job: Job, s: Settings) =>
  extract(job.src, t.glossary.doNotTranslate, { parse: { mdx: s.mdx, mathSingleDollar: s.mathSingleDollar }, docstrings: s.docstrings, codeComments: s.codeComments, frontMatter: s.frontMatter });

const fingerprint = (ex: Extraction) => ex.segments.map((s) => `${s.passive ? 'p' : ''}${s.kind}\u0001${s.masked}\u0001${s.structure ?? ''}`).join('\u0000');

function finish(job: Job, ex: Extraction, tm: Map<string, string>, outcomes: Map<string, Outcome>, s: Settings, started: number, tokens: Run['tokens']): Run {
  const assembled = assembleDocument(ex, tm, { wrap: job.lang.wrap !== 'none', preserveAnchors: s.preserveAnchors });
  const via = [...outcomes.values()];
  return {
    text: assembled.text, reverted: assembled.reverted.length, keptSource: via.filter((o) => o.via === 'source').length, nmt: via.filter((o) => o.via === 'nmt').length,
    reviewEdits: via.filter((o) => o.review).length, seconds: (Date.now() - started) / 1000, tokens, ex, tm, outcomes,
  };
}

/** A chat client whose token usage is metered per run, split into translation and review calls. */
function meteredChat() {
  const tr = new ChatClient(t.cfg, t.auth, translateLimiter);
  const rv = new ChatClient(t.cfg, t.auth, translateLimiter);
  const chat = Object.create(tr) as InstanceType<typeof ChatClient>;
  chat.json = (dep, messages, name, schema, effort) => (name === 'review' ? rv : tr).json(dep, messages, name, schema, effort);
  const sum = (c: InstanceType<typeof ChatClient>) => [...c.usage.byDeployment.values()].reduce((n, u) => n + u.promptTokens + u.completionTokens, 0);
  return { chat, tokens: () => ({ translate: sum(tr), review: sum(rv) }) };
}

async function translate(job: Job, s: Settings, ex?: Extraction): Promise<Run> {
  const started = Date.now();
  const metered = meteredChat();
  try {
    ex ??= extractFor(job, s);
    const { tm, outcomes } = await translateDocument(
      { cfg: t.cfg, chat: metered.chat, nmt: t.nmt, cache: job.cache },
      {
        docName: job.doc, ex, lang: job.lang, sourceLanguage: 'English', sourceLanguageCode: 'en', analysis: job.analysis as never, formality: job.formality,
        glossary: t.glossary, engine: 'gpt', review: s.review, structuralContext: s.structuralContext, nmtFallback: s.nmtFallback,
      },
    );
    return finish(job, ex, tm, outcomes, s, started, metered.tokens());
  } catch (e) {
    // Same behaviour as the API: a file that fails is returned unchanged.
    return { text: job.src, error: (e as Error).message, reverted: 0, keptSource: 0, nmt: 0, reviewEdits: 0, seconds: (Date.now() - started) / 1000, tokens: metered.tokens() };
  }
}

function derive(job: Job, f: Feature, s: Settings): Run {
  const b = job.base!;
  const tm = new Map(b.tm);
  const outcomes = new Map([...b.outcomes!].map(([k, o]) => [k, { ...o }]));
  if (f === 'review') {
    for (const o of outcomes.values()) {
      if (o.review?.before !== undefined) tm.set(o.id, o.review.before);
      delete o.review;
    }
  } else if (f === 'nmtFallback') {
    for (const o of outcomes.values()) if (o.via === 'nmt') { tm.delete(o.id); o.via = 'source'; }
  }
  return finish(job, b.ex!, tm, outcomes, s, Date.now(), f === 'review' ? { translate: b.tokens.translate, review: 0 } : b.tokens);
}

const runPath = (job: Job, variant: string) => join(OUT, job.lang.code, variant, job.doc);
const sha = (s: string) => createHash('sha256').update(s).digest('hex');
// Saved outputs are reused only while the current code extracts the document exactly as before and prompts are unchanged.
const extractionKey = (job: Job, s: Settings) => {
  try {
    return sha(`${PROMPT_VERSION}\u0000${fingerprint(extractFor(job, s))}`);
  } catch {
    return 'error';
  }
};

function save(job: Job, variant: string, r: Run, s: Settings) {
  const p = runPath(job, variant);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, r.text);
  const meta = { fp: extractionKey(job, s), error: r.error, reverted: r.reverted, keptSource: r.keptSource, nmt: r.nmt, reviewEdits: r.reviewEdits, seconds: r.seconds, tokens: r.tokens, reused: r.reused };
  const state = r.tm && variant === 'baseline' ? { tm: [...r.tm], outcomes: [...r.outcomes!.values()] } : undefined;
  writeFileSync(`${p}.meta.json`, JSON.stringify({ ...meta, ...state }));
}

function load(job: Job, variant: string, s: Settings): Run | undefined {
  const p = runPath(job, variant);
  if (!existsSync(p) || !existsSync(`${p}.meta.json`)) return undefined;
  const meta = JSON.parse(readFileSync(`${p}.meta.json`, 'utf8'));
  if (meta.fp !== extractionKey(job, s)) return undefined;
  const r: Run = { text: readFileSync(p, 'utf8'), tokens: { translate: 0, review: 0 }, ...meta };
  if (meta.tm) {
    r.ex = extractFor(job, s);
    r.tm = new Map(meta.tm);
    r.outcomes = new Map((meta.outcomes as Outcome[]).map((o) => [o.id, o]));
  }
  return r;
}

const analyses = new Map<string, { analysis: unknown; formality: 'formal' | 'informal' }>();
const unknown = LANGS.filter((c) => !t.catalog.languages.has(c));
if (unknown.length) throw new Error(`unknown EVAL_LANGS ${unknown.join(', ')}; add them to config/languages.json (known: ${[...t.catalog.languages.keys()].join(', ')})`);
const analysisFile = join(OUT, 'analysis.json');
const savedAnalyses: Record<string, { src: string; analysis: unknown }> = existsSync(analysisFile) ? JSON.parse(readFileSync(analysisFile, 'utf8')) : {};
await Promise.all(
  DOCS.map(async (doc) => {
    const src = readFileSync(join(corpusDir, doc), 'utf8');
    const ex = extract(src, t.glossary.doNotTranslate, { parse: naturalParse(doc) });
    const saved = savedAnalyses[doc]?.src === sha(src) ? savedAnalyses[doc] : undefined;
    const analysis = (saved?.analysis ?? (ex.segments.some((s) => !s.passive) ? await t.analyze(doc, ex.source) : undefined)) as { register?: string } | undefined;
    savedAnalyses[doc] = { src: sha(src), analysis };
    analyses.set(doc, { analysis, formality: ex.frontmatterFormality ?? (analysis?.register === 'informal' ? 'informal' : 'formal') });
  }),
);
mkdirSync(OUT, { recursive: true });
writeFileSync(analysisFile, JSON.stringify(savedAnalyses));
const panel = await createJudges(JUDGES, t.chat);
console.log(`corpus: ${DOCS.length} documents, languages ${LANGS.join(', ')}, features ${SELECTED.join(', ')}, judges ${JUDGES.join(', ')}; output ${OUT}`);

const jobs: Job[] = DOCS.flatMap((doc) =>
  LANGS.map((code) => ({
    doc, lang: t.catalog.languages.get(code)!, src: readFileSync(join(corpusDir, doc), 'utf8'), ...analyses.get(doc)!, cache: new TranslationCache(), variants: {},
  })),
);
let translated = 0;
await Promise.all(
  jobs.map(async (job) => {
    const base = defaults(job.doc);
    const loadedBase = load(job, 'baseline', base);
    job.base = loadedBase ?? (await translate(job, base));
    save(job, 'baseline', job.base, base);
    const baseEx = job.base.ex ?? extractFor(job, base);
    for (const f of SELECTED) {
      const s = { ...base, [f]: !base[f] };
      const name = `${f}=${s[f]}`;
      // Variants depend on the baseline (cache reuse, derivation), so they are reused only with an unchanged baseline.
      let r = loadedBase ? load(job, name, s) : undefined;
      if (!r) {
        if (DERIVED.includes(f) && base[f] && job.base.tm) r = derive(job, f, s);
        else if (f === 'structuralContext' || f === 'review') r = await translate(job, s);
        else {
          let ex: Extraction | undefined;
          try {
            ex = extractFor(job, s);
          } catch {
            ex = undefined;
          }
          // Identical extraction means identical output: no need to call the model again.
          r = ex && fingerprint(ex) === fingerprint(baseEx) ? { ...job.base, seconds: 0, reused: true } : await translate(job, s, ex);
        }
      }
      save(job, name, r, s);
      job.variants[f] = r;
    }
    console.log(`translated ${++translated}/${jobs.length}: ${job.doc} -> ${job.lang.code}${job.base.error ? ` (baseline error: ${job.base.error})` : ''}`);
  }),
);

// ------------------------------------------------------------------ deterministic checks

interface Metrics {
  keepMissing: number;
  translateLeft: number;
  structureBroken: number;
  codeChanged: number;
  linksBroken: number;
  englishLeft: number;
  englishByKind: Record<string, number>;
  reverted: number;
  keptSource: number;
  nmt: number;
  errors: number;
}

function codeBlocks(text: string, opts: ParseOptions) {
  const out: { lang: string; value: string }[] = [];
  const walk = (n: { type: string; lang?: string | null; value?: string; children?: unknown[] }) => {
    if (n.type === 'code') out.push({ lang: n.lang ?? '', value: n.value ?? '' });
    for (const c of (n.children ?? []) as never[]) walk(c);
  };
  walk(parseMarkdown(text, opts) as never);
  return out;
}

function brokenLinks(text: string, opts: ParseOptions): number {
  const tree = parseMarkdown(text, opts);
  const slugger = new GithubSlugger();
  const ids = new Set<string>();
  const targets: string[] = [];
  const walk = (n: { type: string; url?: string; children?: unknown[] }) => {
    if (n.type === 'heading') ids.add(slugger.slug(toString(n as never)));
    if ((n.type === 'link' || n.type === 'definition') && n.url?.startsWith('#')) targets.push(decodeURIComponent(n.url.slice(1)));
    for (const c of (n.children ?? []) as never[]) walk(c);
  };
  walk(tree as never);
  for (const m of text.matchAll(/\b(?:id|name)\s*=\s*["']([^"']+)["']/g)) ids.add(m[1]);
  for (const m of text.matchAll(/\{#([\w-]+)\}/g)) ids.add(m[1]);
  for (const m of text.matchAll(/\bhref\s*=\s*["']#([^"']+)["']/g)) targets.push(m[1]);
  return targets.filter((x) => !ids.has(x)).length;
}

const kindOf = (s: { textContext: string; note?: string }) =>
  s.note?.includes('docstring') ? 'docstring' : s.textContext === 'comment' ? 'comment' : s.textContext === 'yaml' ? 'frontMatter' : s.textContext === 'attr' ? 'attribute' : 'prose';
const plain = (masked: string) => masked.replace(/<\/?[gx]\d+\/?>/g, ' ').replace(/\s+/g, ' ').trim();

/** Segments of the source (all features on) that appear unchanged in the output, i.e. are still English. */
function englishLeft(doc: string, src: string, out: string): Record<string, number> {
  const opts = { parse: naturalParse(doc) };
  const counts: Record<string, number> = {};
  const pool = new Map<string, number>();
  for (const s of extract(src, t.glossary.doNotTranslate, opts).segments) {
    const p = plain(s.masked);
    if (s.passive || (p.match(/\p{L}{2,}/gu)?.length ?? 0) < 3) continue;
    pool.set(`${kindOf(s)}\u0000${p}`, (pool.get(`${kindOf(s)}\u0000${p}`) ?? 0) + 1);
  }
  const stripped = out.replace(/<a id="[^"]*"><\/a>/g, (m) => (src.includes(m) ? m : ''));
  let ex: Extraction;
  try {
    ex = extract(stripped, t.glossary.doNotTranslate, opts);
  } catch {
    return { unparsable: 1 };
  }
  for (const s of ex.segments) {
    const key = `${kindOf(s)}\u0000${plain(s.masked)}`;
    const n = pool.get(key);
    if (!n) continue;
    pool.set(key, n - 1);
    counts[kindOf(s)] = (counts[kindOf(s)] ?? 0) + 1;
  }
  return counts;
}

function metrics(doc: string, src: string, r: Run): Metrics {
  const e = expectations[doc] ?? { keep: [], translate: [] };
  const opts = naturalParse(doc);
  const out = r.text;
  let structureBroken = 0;
  let codeChanged = 0;
  let linksBroken = 0;
  try {
    structureBroken = compareSkeletons(skeleton(src, opts), skeleton(out, opts)) === -1 ? 0 : 1;
    const a = codeBlocks(src, opts);
    const b = codeBlocks(out, opts);
    if (a.length !== b.length) codeChanged = Math.max(a.length, b.length);
    else a.forEach((c, i) => {
      const fa = codeFingerprint(c.lang, c.value);
      const fb = codeFingerprint(b[i].lang, b[i].value);
      if (b[i].lang !== c.lang || (fa === null ? c.value !== b[i].value : fa !== fb)) codeChanged++;
    });
    linksBroken = Math.max(0, brokenLinks(out, opts) - brokenLinks(src, opts));
  } catch {
    structureBroken = 1;
  }
  const englishByKind = englishLeft(doc, src, out);
  return {
    keepMissing: e.keep.filter((k) => !out.includes(k)).length,
    translateLeft: e.translate.filter((p) => out.includes(p)).length,
    structureBroken, codeChanged, linksBroken, englishByKind,
    englishLeft: Object.values(englishByKind).reduce((n, x) => n + x, 0),
    reverted: r.reverted, keptSource: r.keptSource, nmt: r.nmt, errors: r.error ? 1 : 0,
  };
}

// ------------------------------------------------------------------ judge

/** Blocks separated by blank lines; fenced code and front matter stay whole. */
function chunks(text: string): string[] {
  const lines = text.split(/(?<=\n)/);
  const out: string[] = [];
  let cur = '';
  let fence = '';
  let i = 0;
  if (/^---\r?\n/.test(text)) {
    cur = lines[0];
    for (i = 1; i < lines.length; i++) {
      cur += lines[i];
      if (/^(?:---|\.\.\.)\s*$/.test(lines[i])) break;
    }
    out.push(cur);
    cur = '';
    i++;
  }
  for (; i < lines.length; i++) {
    const line = lines[i];
    const body = line.replace(/\r?\n$/, '');
    const inner = body.replace(/^\s*(?:>\s?)*/, '');
    if (fence) {
      cur += line;
      if (new RegExp(`^${fence[0] === '`' ? '`' : '~'}{${fence.length},}\\s*$`).test(inner)) fence = '';
      continue;
    }
    const open = /^(`{3,}|~{3,})/.exec(inner);
    if (open) {
      fence = open[1];
      cur += line;
      continue;
    }
    if (!body.trim()) {
      if (cur) out.push(cur);
      cur = '';
      continue;
    }
    cur += line;
  }
  if (cur) out.push(cur);
  return out;
}

const JUDGE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['results'],
  properties: {
    results: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['id', 'errorsA', 'errorsB', 'better'],
        properties: { id: { type: 'string' }, errorsA: { $ref: '#/$defs/errors' }, errorsB: { $ref: '#/$defs/errors' }, better: { type: 'string', enum: ['A', 'B', 'tie'] } },
      },
    },
  },
  $defs: {
    errors: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['category', 'severity', 'note'],
        properties: {
          category: {
            type: 'string',
            enum: ['mistranslation', 'omission', 'addition', 'terminology', 'grammar', 'spelling', 'punctuation', 'fluency', 'style', 'register', 'locale', 'untranslated', 'overtranslation', 'markup'],
          },
          severity: { type: 'string', enum: ['minor', 'major', 'critical'] },
          note: { type: 'string' },
        },
      },
    },
  },
} as const;

type JudgeError = { category: string; severity: keyof typeof WEIGHT; note: string };
interface JudgeResult {
  results: { id: string; errorsA: JudgeError[]; errorsB: JudgeError[]; better: 'A' | 'B' | 'tie' }[];
}
interface Item {
  id: string;
  source: string;
  on: string;
  off: string;
}

const CATEGORIES = new Set<string>(JUDGE_SCHEMA.$defs.errors.items.properties.category.enum);
const SEVERITIES = new Set(Object.keys(WEIGHT));

/** Checks an answer covers every item with well-formed fields (Copilot answers are not schema-enforced). */
function validateJudgement(items: Item[]) {
  return (v: unknown): string | undefined => {
    const results = (v as JudgeResult | undefined)?.results;
    if (!Array.isArray(results)) return 'missing "results" array';
    const byId = new Map(results.map((r) => [r?.id, r]));
    for (const it of items) {
      const r = byId.get(it.id);
      if (!r) return `missing result for item "${it.id}"`;
      if (!['A', 'B', 'tie'].includes(r.better)) return `item "${it.id}": "better" must be A, B or tie`;
      for (const errs of [r.errorsA, r.errorsB]) {
        if (!Array.isArray(errs)) return `item "${it.id}": errorsA and errorsB must be arrays`;
        for (const e of errs) if (!CATEGORIES.has(e?.category) || !SEVERITIES.has(e?.severity)) return `item "${it.id}": invalid error category or severity`;
      }
    }
    return undefined;
  };
}

async function judge(job: Job, judgeImpl: Judge, items: Item[], swap: boolean, full: { on: string; off: string }) {
  const lang = job.lang;
  const rules = `Rules of this project; a violation is an error:
- Human-readable text must be translated: prose, headings, list items, table cells, link text, image alt text and titles, prose values in YAML front matter (title, description, summary, keywords), comments and docstrings inside code blocks, prose attributes of components. English text left in place where a ${lang.name} reader needs a translation is an "untranslated" error (major).
- Everything else must stay byte-identical: Markdown, HTML and JSX syntax, tag and component names, non-prose attributes, URLs, anchors, file names and paths (also in backticks), code (only its comments and docstrings change), identifiers, commands and flags, environment variables, math formulas, front matter keys and non-prose values, directive and shortcode names, docstring section headers and parameter names. Changing these is "overtranslation" or "markup" (critical when it breaks the document, the code or a link).
- Empty anchors such as <a id="old-slug"></a> before heading text are inserted on purpose; ignore them. Ignore line wrapping differences.
Register expected for this document: ${job.formality === 'informal' ? lang.informal : lang.formal}
Locale style: ${lang.style ?? ''}`;
  const system = `You are a senior ${lang.name} translator and localization QA lead. You review Markdown technical documentation translated from English into ${lang.name}.
For each item you get an English Markdown block and two candidate translations, A and B, as raw Markdown.
${rules}
Annotate every error in each candidate with category and severity (minor: small imperfection; major: changes meaning, clearly wrong or left untranslated; critical: misleading, broken or unusable). Then decide which candidate is better overall ("tie" if equal). Be strict and consistent; do not reward length.`;
  const payload = items.map((it) => ({ id: it.id, source: it.source, A: swap ? it.off : it.on, B: swap ? it.on : it.off }));
  const request = {
    system,
    user: JSON.stringify({ document: job.doc, items: payload }),
    // Blinded like the items: candidate-A is whatever A is in this call.
    documents: { 'source.md': job.src, 'candidate-A.md': swap ? full.off : full.on, 'candidate-B.md': swap ? full.on : full.off, 'guidelines.md': rules },
  };
  const cacheFile = join(OUT, 'judge-cache', `${sha(JSON.stringify([judgeImpl.id, request]))}.json`);
  let res: JudgeResult;
  if (existsSync(cacheFile)) res = JSON.parse(readFileSync(cacheFile, 'utf8'));
  else {
    res = await judgeImpl.json<JudgeResult>({ ...request, schemaName: 'mqm', schema: JUDGE_SCHEMA, validate: validateJudgement(items) });
    mkdirSync(dirname(cacheFile), { recursive: true });
    writeFileSync(cacheFile, JSON.stringify(res));
  }
  type R = JudgeResult['results'][number];
  const flip = (b: R['better']): R['better'] => (b === 'A' ? 'B' : b === 'B' ? 'A' : 'tie');
  return new Map(res.results.map((r): [string, R] => [r.id, swap ? { ...r, errorsA: r.errorsB, errorsB: r.errorsA, better: flip(r.better) } : r]));
}

interface Tally {
  comparison: string;
  judge: string;
  lang: string;
  doc: string;
  items: number;
  wins: number;
  losses: number;
  ties: number;
  inconsistent: number;
  mqmOn: number;
  mqmOff: number;
  words: number;
}
const tallies: Tally[] = [];
const examples: unknown[] = [];
const judgements: unknown[] = [];
const words = (s: string) => s.split(/\s+/).filter((w) => /\p{L}/u.test(w)).length;
const penalty = (errs: JudgeError[]) => errs.reduce((n, e) => n + (WEIGHT[e.severity] ?? 1), 0);

const judgeTasks: (() => Promise<void>)[] = [];
const judgeLabels: string[] = [];
for (const job of jobs) {
  const srcChunks = chunks(job.src);
  for (const f of SELECTED) {
    if (NOT_JUDGED.includes(f)) continue;
    const base = defaults(job.doc);
    const on = base[f] ? job.base! : job.variants[f]!;
    const off = base[f] ? job.variants[f]! : job.base!;
    if (on.text === off.text) continue;
    const full = { on: on.text, off: off.text };
    let a = chunks(on.text);
    let b = chunks(off.text);
    let s = srcChunks;
    if (a.length !== s.length || b.length !== s.length) [s, a, b] = [[job.src], [on.text], [off.text]];
    const items = s.map((source, i) => ({ id: `b${i}`, source, on: a[i], off: b[i] })).filter((it) => it.on !== it.off);
    const batches: Item[][] = [];
    let cur: Item[] = [];
    let size = 0;
    for (const it of items) {
      const n = it.source.length + it.on.length + it.off.length;
      if (cur.length && (cur.length >= 8 || size + n > 16000)) {
        batches.push(cur);
        cur = [];
        size = 0;
      }
      cur.push(it);
      size += n;
    }
    if (cur.length) batches.push(cur);
    for (const judgeImpl of panel.judges) {
      const judgeDep = judgeImpl.id;
      const tally: Tally = { comparison: comparisonOf(f, job.doc), judge: judgeDep, lang: job.lang.code, doc: job.doc, items: items.length, wins: 0, losses: 0, ties: 0, inconsistent: 0, mqmOn: 0, mqmOff: 0, words: items.reduce((n, it) => n + words(it.source), 0) };
      tallies.push(tally);
      for (const batch of batches) {
        judgeLabels.push(`${judgeDep} ${f} ${job.doc} ${job.lang.code}`);
        judgeTasks.push(async () => {
          let r1: Map<string, JudgeResult['results'][number]>;
          let r2: Map<string, JudgeResult['results'][number]>;
          try {
            [r1, r2] = await Promise.all([judge(job, judgeImpl, batch, false, full), judge(job, judgeImpl, batch, true, full)]);
          } catch (e) {
            console.error(`judge failed ${judgeDep} ${f} ${job.doc} ${job.lang.code}: ${(e as Error).message}`);
            tally.ties += batch.length;
            for (const it of batch) judgements.push({ comparison: tally.comparison, feature: f, doc: job.doc, lang: job.lang.code, judge: judgeDep, item: it.id, words: words(it.source), verdict: 'error' });
            return;
          }
          for (const it of batch) {
            const x = r1.get(it.id);
            const y = r2.get(it.id);
            if (!x || !y) {
              tally.ties++;
              judgements.push({ comparison: tally.comparison, feature: f, doc: job.doc, lang: job.lang.code, judge: judgeDep, item: it.id, words: words(it.source), verdict: 'error' });
              continue;
            }
            const side = (b: string) => (b === 'A' ? 'on' : b === 'B' ? 'off' : 'tie');
            const verdict = x.better === y.better ? side(x.better) : x.better !== 'tie' && y.better !== 'tie' ? 'inconsistent' : 'tie';
            const brief = (errs: JudgeError[]) => errs.map((er) => ({ category: er.category, severity: er.severity }));
            judgements.push({
              comparison: tally.comparison, feature: f, doc: job.doc, lang: job.lang.code, judge: judgeDep, item: it.id, words: words(it.source),
              order1: side(x.better), order2: side(y.better), verdict,
              mqmOn: (penalty(x.errorsA) + penalty(y.errorsA)) / 2, mqmOff: (penalty(x.errorsB) + penalty(y.errorsB)) / 2,
              errorsOn: [...brief(x.errorsA), ...brief(y.errorsA)], errorsOff: [...brief(x.errorsB), ...brief(y.errorsB)],
            });
            tally.mqmOn += (penalty(x.errorsA) + penalty(y.errorsA)) / 2;
            tally.mqmOff += (penalty(x.errorsB) + penalty(y.errorsB)) / 2;
            if (x.better === y.better && x.better === 'A') tally.wins++;
            else if (x.better === y.better && x.better === 'B') tally.losses++;
            else {
              tally.ties++;
              if (x.better !== y.better && x.better !== 'tie' && y.better !== 'tie') tally.inconsistent++;
            }
            if (examples.length < 600 && x.better === y.better && x.better !== 'tie') {
              examples.push({ comparison: tally.comparison, judge: judgeDep, lang: job.lang.code, doc: job.doc, winner: x.better === 'A' ? 'on' : 'off', source: it.source, on: it.on, off: it.off, errorsOn: x.errorsA, errorsOff: x.errorsB });
            }
          }
        });
      }
    }
  }
}
let judged = 0;
const pending = new Set(judgeLabels.map((l, i) => `${i}: ${l}`));
const heartbeat = setInterval(() => {
  if (pending.size && pending.size <= 10) console.log(`still judging ${pending.size}: ${[...pending].map((p) => p.replace(/^\d+: /, '')).join('; ')}`);
}, 60000);
await Promise.all(
  judgeTasks.map(async (task, i) => {
    await task();
    pending.delete(`${i}: ${judgeLabels[i]}`);
    if (++judged % 20 === 0 || judged === judgeTasks.length || pending.size <= 10) console.log(`judged ${judged}/${judgeTasks.length} batches`);
  }),
);
clearInterval(heartbeat);
await panel.close();

// ------------------------------------------------------------------ report

function binomialP(k: number, n: number): number {
  if (n === 0) return 1;
  const lo = Math.min(k, n - k);
  let p = 0;
  for (let i = 0; i <= lo; i++) {
    let c = 1;
    for (let j = 0; j < i; j++) c = (c * (n - j)) / (j + 1);
    p += c / 2 ** n;
  }
  return Math.min(1, 2 * p);
}

function summarize(ts: Tally[]) {
  const sum = (k: keyof Tally) => ts.reduce((n, x) => n + (x[k] as number), 0);
  const w = sum('wins');
  const l = sum('losses');
  const per100 = Math.max(1, sum('words') / 100);
  return {
    items: sum('items'), wins: w, losses: l, ties: sum('ties'), inconsistent: sum('inconsistent'), pSignTest: +binomialP(w, w + l).toFixed(4),
    mqmOn: +(sum('mqmOn') / per100).toFixed(2), mqmOff: +(sum('mqmOff') / per100).toFixed(2),
  };
}

const CORRECTNESS: (keyof Metrics)[] = ['keepMissing', 'translateLeft', 'structureBroken', 'codeChanged', 'linksBroken', 'englishLeft', 'errors'];
const addMetrics = (a: Metrics, b: Metrics): Metrics => {
  const out = { ...a, englishByKind: { ...a.englishByKind } };
  for (const k of Object.keys(b) as (keyof Metrics)[]) if (k !== 'englishByKind') (out[k] as number) += b[k] as number;
  for (const [k, v] of Object.entries(b.englishByKind)) out.englishByKind[k] = (out.englishByKind[k] ?? 0) + v;
  return out;
};
const zero = (): Metrics => ({ keepMissing: 0, translateLeft: 0, structureBroken: 0, codeChanged: 0, linksBroken: 0, englishLeft: 0, englishByKind: {}, reverted: 0, keptSource: 0, nmt: 0, errors: 0 });
const problems = (m: Metrics) => CORRECTNESS.reduce((n, k) => n + (m[k] as number), 0);

const comparisons = [...new Set(jobs.flatMap((j) => SELECTED.map((f) => comparisonOf(f, j.doc))))];
const summary: Record<string, unknown> = {};
const perDoc: unknown[] = [];
const records: unknown[] = [];
const sideOf = (r: Run, m: Metrics) => ({ ...m, seconds: r.seconds, tokens: r.tokens, reused: !!r.reused });
for (const c of comparisons) {
  const f = (c.startsWith('mdx') ? 'mdx' : c) as Feature;
  const rel = jobs.filter((j) => comparisonOf(f, j.doc) === c);
  let on = zero();
  let off = zero();
  let affected = 0;
  let secondsOn = 0;
  let secondsOff = 0;
  for (const j of rel) {
    const base = defaults(j.doc);
    const rOn = base[f] ? j.base! : j.variants[f]!;
    const rOff = base[f] ? j.variants[f]! : j.base!;
    const mOn = metrics(j.doc, j.src, rOn);
    const mOff = metrics(j.doc, j.src, rOff);
    on = addMetrics(on, mOn);
    off = addMetrics(off, mOff);
    secondsOn += rOn.seconds;
    secondsOff += rOff.seconds;
    records.push({ comparison: c, feature: f, doc: j.doc, lang: j.lang.code, defaultOn: base[f], changed: rOn.text !== rOff.text, on: sideOf(rOn, mOn), off: sideOf(rOff, mOff) });
    if (rOn.text !== rOff.text) {
      affected++;
      perDoc.push({ comparison: c, doc: j.doc, lang: j.lang.code, problemsOn: problems(mOn), problemsOff: problems(mOff), on: mOn, off: mOff });
    }
  }
  const ts = tallies.filter((x) => x.comparison === c);
  const judgedSummary = summarize(ts);
  const decided = judgedSummary.wins + judgedSummary.losses;
  const quality = NOT_JUDGED.includes(f) ? 'not judged' : !decided ? 'no judged difference' : judgedSummary.pSignTest < 0.05 ? (judgedSummary.wins > judgedSummary.losses ? 'significantly better' : 'significantly worse') : 'no significant difference';
  const pOn = problems(on);
  const pOff = problems(off);
  const correctness = pOn < pOff ? 'fewer problems' : pOn > pOff ? 'more problems' : 'same';
  const recommendation = quality === 'significantly better' || (correctness === 'fewer problems' && quality !== 'significantly worse') ? 'on' : 'off';
  summary[c] = {
    documents: rel.length, affected, quality, correctness, recommendation,
    problemsOn: pOn, problemsOff: pOff, on, off,
    avgSecondsOn: +(secondsOn / Math.max(1, rel.length)).toFixed(1), avgSecondsOff: +(secondsOff / Math.max(1, rel.length)).toFixed(1),
    judged: { overall: judgedSummary, byJudge: Object.fromEntries(JUDGES.map((jd) => [jd, summarize(ts.filter((x) => x.judge === jd))])), byLanguage: Object.fromEntries(LANGS.map((l) => [l, summarize(ts.filter((x) => x.lang === l))])) },
  };
}

mkdirSync(OUT, { recursive: true });
const defects = jobs.map((j) => {
  const e = expectations[j.doc] ?? { keep: [], translate: [] };
  const m = metrics(j.doc, j.src, j.base!);
  return {
    doc: j.doc, lang: j.lang.code, error: j.base!.error, keepMissing: e.keep.filter((k) => !j.base!.text.includes(k)), translateLeft: e.translate.filter((p) => j.base!.text.includes(p)),
    structureBroken: m.structureBroken, codeChanged: m.codeChanged, linksBroken: m.linksBroken, englishByKind: m.englishByKind, reverted: m.reverted, keptSource: m.keptSource, nmt: m.nmt,
    tokens: j.base!.tokens, seconds: j.base!.seconds,
  };
});
let commit = '';
try {
  commit = execSync('git rev-parse --short HEAD', { encoding: 'utf8' }).trim() + (execSync('git status --porcelain -- src', { encoding: 'utf8' }).trim() ? '+dirty' : '');
} catch {
  commit = 'unknown';
}
const meta = {
  createdAt: new Date().toISOString(), commit, langs: LANGS, docs: DOCS, judges: JUDGES, judgePanel: panel.judges.map((j) => ({ id: j.id, provider: j.provider, model: j.model, family: j.family })), features: SELECTED,
  config: { translateDeployment: cfg.translateDeployment, reviewDeployment: cfg.reviewDeployment, translateReasoning: cfg.translateReasoning, reviewReasoning: cfg.reviewReasoning, defaults: defaults('x.md') },
};
const resultsFile = join(OUT, 'features.json');
writeFileSync(resultsFile, JSON.stringify({ meta, summary, perDoc, records, judgements, defects, tallies, usage: t.chat.usage.toJSON(), examples }, null, 2));

const rows = Object.entries(summary).map(([c, v]) => {
  const s = v as { affected: number; documents: number; quality: string; correctness: string; recommendation: string; problemsOn: number; problemsOff: number; judged: { overall: ReturnType<typeof summarize> } };
  const o = s.judged.overall;
  return { feature: c, affected: `${s.affected}/${s.documents}`, 'won/lost': `${o.wins}/${o.losses}`, p: o.pSignTest, 'mqm on/off': `${o.mqmOn}/${o.mqmOff}`, 'problems on/off': `${s.problemsOn}/${s.problemsOff}`, quality: s.quality, correctness: s.correctness, recommendation: s.recommendation };
});
console.log('\n=== feature impact (on vs off; problems = expectation misses, structure, code, links, English left, errors) ===');
console.table(rows);
const md = [
  `# Feature evaluation`,
  '',
  `${DOCS.length} documents, languages ${LANGS.join(', ')}, judges ${JUDGES.join(', ')}.`,
  '',
  '| Feature | Affected | Won / lost | p | MQM on / off | Problems on / off | Quality | Correctness | Recommendation |',
  '|---|---|---|---|---|---|---|---|---|',
  ...rows.map((r) => `| ${r.feature} | ${r.affected} | ${r['won/lost']} | ${r.p} | ${r['mqm on/off']} | ${r['problems on/off']} | ${r.quality} | ${r.correctness} | ${r.recommendation} |`),
  '',
].join('\n');
writeFileSync(join(OUT, 'report.md'), md);
console.log(`\nresults: ${resultsFile}\nquick report: ${join(OUT, 'report.md')}`);
if (process.env.EVAL_REPORT !== 'off') {
  const { buildReport } = await import('./report.js');
  const dir = buildReport(resultsFile, process.env.EVAL_REPORT_DIR);
  console.log(`evaluation report: ${join(dir, 'README.md')}`);
}
