/**
 * Measures translation quality of pipeline variants with blind, position-swapped pairwise judging (MQM-style).
 * The variants come from the implementation under test (the Rust binary by default, the TypeScript pipeline with
 * EVAL_IMPL=typescript, see eval/implementation.ts); judging uses the Foundry chat client.
 *
 *   cargo build --release --manifest-path rust/Cargo.toml    # for the Rust binary
 *   npx tsx eval/run.ts                      # defaults below
 *   EVAL_LANGS=de,fr EVAL_DOCS=blog.md npx tsx eval/run.ts
 */
import { existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

if (existsSync('.env')) process.loadEnvFile('.env');
delete process.env.MDT_CACHE_DIR;

const { loadConfig } = await import('../src/config.js');
const { ChatClient } = await import('../src/azure/openai.js');
const { AzureAuth, Semaphore } = await import('../src/azure/http.js');
const impl = await import('./implementation.js');
type Outcome = import('./implementation.js').Outcome;
type Segment = import('./implementation.js').SegmentInfo;

const cfg = loadConfig();
const judgeChat = new ChatClient(cfg, new AzureAuth(cfg), new Semaphore(cfg.maxConcurrency));
const catalog = await impl.languages();

const LANGS = (process.env.EVAL_LANGS ?? 'de,fr,sv,pl,fi,mt').split(',');
const PRIMARY = process.env.EVAL_PRIMARY ?? 'translate';
const ALT = process.env.EVAL_ALT ?? 'translate-alt';
const JUDGES = (process.env.EVAL_JUDGES ?? `${ALT},${PRIMARY}`).split(',');
const corpusDir = new URL('./corpus/', import.meta.url);
const DOCS = (process.env.EVAL_DOCS?.split(',') ?? readdirSync(corpusDir).filter((f) => f.endsWith('.md'))).sort();

type Variant = 'nmt' | 'gpt' | 'gpt+review' | 'alt' | 'noctx';
const ALL_VARIANTS: Variant[] = ['nmt', 'gpt', 'gpt+review', 'alt', 'noctx'];
const VARIANTS = (process.env.EVAL_VARIANTS?.split(',') as Variant[] | undefined) ?? ALL_VARIANTS;
// 'gpt' sends structural context; 'noctx' is the same pipeline without it.
const COMPARISONS = ([
  ['gpt', 'nmt'],
  ['gpt+review', 'gpt'],
  ['alt', 'gpt'],
  ['gpt', 'noctx'],
] as [Variant, Variant][]).filter(([x, y]) => VARIANTS.includes(x) && VARIANTS.includes(y));
const WEIGHT = { minor: 1, major: 5, critical: 10 } as const;

interface Run {
  tm: Map<string, string>;
  outcomes: Map<string, Outcome>;
  /** Each segment rendered on its own (single line), as produced by the implementation. */
  rendered: Map<string, string>;
  reverted: number;
  keptSource: number;
  seconds: number;
}

interface Job {
  doc: string;
  file: string;
  lang: string;
  segs: Segment[];
  analysis: unknown;
  runs: Partial<Record<Variant, Run>>;
  formality: 'formal' | 'informal';
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
        properties: {
          id: { type: 'string' },
          errorsA: { $ref: '#/$defs/errors' },
          errorsB: { $ref: '#/$defs/errors' },
          better: { type: 'string', enum: ['A', 'B', 'tie'] },
        },
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
          category: { type: 'string', enum: ['mistranslation', 'omission', 'addition', 'terminology', 'grammar', 'spelling', 'punctuation', 'fluency', 'style', 'register', 'locale', 'untranslated'] },
          severity: { type: 'string', enum: ['minor', 'major', 'critical'] },
          note: { type: 'string' },
        },
      },
    },
  },
} as const;

interface JudgeResult {
  results: { id: string; errorsA: { category: string; severity: keyof typeof WEIGHT; note: string }[]; errorsB: { category: string; severity: keyof typeof WEIGHT; note: string }[]; better: 'A' | 'B' | 'tie' }[];
}

const oneLine = (s: string) => s.replace(/\s*\r?\n\s*/g, ' ');
const render = (job: Job, seg: Segment, v: Variant) => oneLine(job.runs[v]!.rendered.get(seg.id) ?? seg.original);
const words = (s: string) => s.split(/\s+/).filter((w) => /\p{L}/u.test(w)).length;

async function runVariant(job: Job, v: Variant): Promise<Run> {
  const started = Date.now();
  const common = { formality: job.formality, analysis: job.analysis ?? null, sourceLanguage: 'en', reportSegments: true } as const;
  let rep: import('./implementation.js').Report;
  if (v === 'gpt+review') {
    // Review pass over the gpt translations, as a separate step.
    const g = job.runs.gpt!;
    rep = await impl.translate(job.file, job.lang, { review: true }, { ...common, memory: { tm: [...g.tm], outcomes: [...g.outcomes.values()], review: true } });
  } else {
    rep = await impl.translate(job.file, job.lang, { review: false, structuralContext: v !== 'noctx' }, { ...common, engine: v === 'nmt' ? 'nmt' : 'gpt', translateDeployment: v === 'alt' ? ALT : PRIMARY });
  }
  const l = rep.languages[0];
  const outcomes = new Map(l.outcomes.map((o) => [o.id, o]));
  return {
    tm: new Map(l.tm),
    outcomes,
    rendered: new Map((l.segmentsDetail ?? []).map((s) => [s.id, s.rendered])),
    reverted: l.revertedForStructure.length,
    keptSource: [...outcomes.values()].filter((o) => o.via === 'source').length,
    seconds: (Date.now() - started) / 1000,
  };
}

async function judge(job: Job, judgeDeployment: string, a: Variant, b: Variant, segs: Segment[]) {
  const lang = catalog.languages.get(job.lang.toLowerCase())!;
  const system = `You are a senior ${lang.name} translator and localization QA lead. You evaluate translations of technical documentation from English into ${lang.name} using MQM error annotation.
For each segment you get the English source and two candidate translations, A and B. Markup, code and placeholders are identical by construction; judge only the language.
Annotate every error in each candidate with category and severity (minor: small imperfection; major: changes meaning or clearly wrong/unnatural; critical: misleading or unusable). Then decide which candidate is better overall ("tie" if their quality is equal).
Register expected for this document: ${job.formality === 'informal' ? lang.informal : lang.formal}
Locale style: ${lang.style ?? ''}
Be strict, precise and consistent. Do not reward length or literalness.`;
  const payload = segs.map((s) => ({ id: s.id, kind: s.kind, source: oneLine(s.original), A: render(job, s, a), B: render(job, s, b) }));
  const res = await judgeChat.json<JudgeResult>(
    judgeDeployment,
    [
      { role: 'system', content: system },
      { role: 'user', content: JSON.stringify({ document: job.doc, segments: payload }) },
    ],
    'mqm',
    JUDGE_SCHEMA,
    'high',
  );
  return res.results;
}

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

// ------------------------------------------------------------------ translate

const unknownLangs = LANGS.filter((c) => !catalog.languages.has(c.toLowerCase()));
if (unknownLangs.length) throw new Error(`unknown EVAL_LANGS ${unknownLangs.join(', ')}`);
const jobs: Job[] = [];
for (const doc of DOCS) {
  const file = fileURLToPath(new URL(doc, corpusDir));
  const ex = await impl.extraction(file, {});
  const segs = ex.segments.filter((s) => !s.passive);
  const analysis = (segs.length ? await impl.analyze(file) : undefined) as { register?: string } | undefined;
  const formality = ex.frontmatterFormality ?? (analysis?.register === 'informal' ? 'informal' : 'formal');
  console.log(`${doc}: ${segs.length} segments, register=${analysis?.register} -> ${formality}`);
  for (const code of LANGS) jobs.push({ doc, file, lang: code, segs, analysis, runs: {}, formality });
}

await Promise.all(
  jobs.map(async (job) => {
    const first = VARIANTS.filter((v) => v !== 'gpt+review');
    const runs = await Promise.all(first.map((v) => runVariant(job, v)));
    first.forEach((v, i) => (job.runs[v] = runs[i]));
    if (VARIANTS.includes('gpt+review')) job.runs['gpt+review'] = await runVariant(job, 'gpt+review');
    console.log(`translated ${job.doc} -> ${job.lang}`);
  }),
);

// ------------------------------------------------------------------ judge

interface Tally {
  comparison: string;
  judge: string;
  lang: string;
  doc: string;
  segments: number;
  differing: number;
  wins: number;
  losses: number;
  ties: number;
  inconsistent: number;
  mqmA: number;
  mqmB: number;
  words: number;
}
const tallies: Tally[] = [];
const examples: unknown[] = [];
const judgeTotal = jobs.length * COMPARISONS.length * JUDGES.length;
let judgeDone = 0;

await Promise.all(
  jobs.flatMap((job) =>
    COMPARISONS.flatMap(([x, y]) =>
      JUDGES.map(async (judgeDep) => {
        const differing = job.segs.filter((s) => render(job, s, x) !== render(job, s, y));
        const tally: Tally = {
          comparison: `${x} vs ${y}`, judge: judgeDep, lang: job.lang, doc: job.doc, segments: job.segs.length, differing: differing.length,
          wins: 0, losses: 0, ties: job.segs.length - differing.length, inconsistent: 0, mqmA: 0, mqmB: 0, words: job.segs.reduce((n, s) => n + words(s.original), 0),
        };
        tallies.push(tally);
        for (let i = 0; i < differing.length; i += 20) {
          const batch = differing.slice(i, i + 20);
          let first: JudgeResult['results'];
          let second: JudgeResult['results'];
          try {
            [first, second] = await Promise.all([judge(job, judgeDep, x, y, batch), judge(job, judgeDep, y, x, batch)]);
          } catch (e) {
            console.error(`judge failed ${judgeDep} ${job.doc} ${job.lang}: ${(e as Error).message}`);
            tally.ties += batch.length;
            continue;
          } finally {
            if (i + 20 >= differing.length) console.log(`judged ${++judgeDone}/${judgeTotal}: ${tally.comparison} ${judgeDep} ${job.doc} ${job.lang}`);
          }
          const f = new Map(first.map((r) => [r.id, r]));
          const s2 = new Map(second.map((r) => [r.id, r]));
          for (const seg of batch) {
            const r1 = f.get(seg.id);
            const r2 = s2.get(seg.id);
            if (!r1 || !r2) {
              tally.ties++;
              continue;
            }
            const penalty = (errs: { severity: keyof typeof WEIGHT }[]) => errs.reduce((n, e) => n + (WEIGHT[e.severity] ?? 1), 0);
            tally.mqmA += (penalty(r1.errorsA) + penalty(r2.errorsB)) / 2;
            tally.mqmB += (penalty(r1.errorsB) + penalty(r2.errorsA)) / 2;
            const p1 = r1.better === 'A' ? x : r1.better === 'B' ? y : 'tie';
            const p2 = r2.better === 'A' ? y : r2.better === 'B' ? x : 'tie';
            if (p1 === p2 && p1 === x) tally.wins++;
            else if (p1 === p2 && p1 === y) tally.losses++;
            else {
              tally.ties++;
              if (p1 !== p2 && p1 !== 'tie' && p2 !== 'tie') tally.inconsistent++;
            }
            if (examples.length < 400 && p1 === p2 && p1 !== 'tie') {
              examples.push({ comparison: tally.comparison, judge: judgeDep, lang: job.lang, id: seg.id, source: oneLine(seg.original), [x]: render(job, seg, x), [y]: render(job, seg, y), winner: p1, errorsX: r1.errorsA, errorsY: r1.errorsB });
            }
          }
        }
      }),
    ),
  ),
);

// ------------------------------------------------------------------ report

function summarize(filter: (t: Tally) => boolean) {
  const ts = tallies.filter(filter);
  const sum = (k: keyof Tally) => ts.reduce((n, x) => n + (x[k] as number), 0);
  const w = sum('wins');
  const l = sum('losses');
  const words100 = sum('words') / 100;
  return {
    segments: sum('segments'), differing: sum('differing'), wins: w, losses: l, ties: sum('ties'), inconsistent: sum('inconsistent'),
    winRate: +(w / Math.max(1, w + l)).toFixed(3), pSignTest: +binomialP(w, w + l).toFixed(4),
    mqmPer100WordsA: +(sum('mqmA') / Math.max(1, words100)).toFixed(2), mqmPer100WordsB: +(sum('mqmB') / Math.max(1, words100)).toFixed(2),
  };
}

const summary: Record<string, unknown> = {};
for (const [x, y] of COMPARISONS) {
  const c = `${x} vs ${y}`;
  summary[c] = {
    overall: summarize((t) => t.comparison === c),
    byJudge: Object.fromEntries(JUDGES.map((j) => [j, summarize((t) => t.comparison === c && t.judge === j)])),
    byLanguage: Object.fromEntries(LANGS.map((l) => [l, summarize((t) => t.comparison === c && t.lang === l)])),
    byDoc: Object.fromEntries(DOCS.map((d) => [d, summarize((t) => t.comparison === c && t.doc === d)])),
  };
}
const structure = Object.fromEntries(
  VARIANTS.map((v) => [
    v,
    {
      reverted: jobs.reduce((n, j) => n + j.runs[v]!.reverted, 0),
      keptSource: jobs.reduce((n, j) => n + j.runs[v]!.keptSource, 0),
      retried: jobs.reduce((n, j) => n + [...j.runs[v]!.outcomes.values()].filter((o) => o.retries > 0).length, 0),
      nmtFallback: jobs.reduce((n, j) => n + [...j.runs[v]!.outcomes.values()].filter((o) => o.via === 'nmt').length, 0),
      avgSecondsPerDoc: +(jobs.reduce((n, j) => n + j.runs[v]!.seconds, 0) / jobs.length).toFixed(1),
    },
  ]),
);
const reviewEdits = jobs.reduce((n, j) => n + [...(j.runs['gpt+review']?.outcomes.values() ?? [])].filter((o) => o.review).length, 0);

mkdirSync(new URL('./results/', import.meta.url), { recursive: true });
const file = join(new URL('./results/', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'), `eval-${new Date().toISOString().replace(/[:.]/g, '-')}.json`);
writeFileSync(file, JSON.stringify({ langs: LANGS, docs: DOCS, judges: JUDGES, primary: PRIMARY, alt: ALT, implementation: await impl.version(), summary, structure, reviewEdits, usage: { translator: impl.usage(), judges: judgeChat.usage.toJSON() }, tallies, examples }, null, 2));

console.log('\n=== structure ===');
console.table(structure);
console.log(`review pass changed ${reviewEdits} segments`);
for (const [c, s] of Object.entries(summary)) {
  console.log(`\n=== ${c} (winRate = share of decided segments won by the first variant) ===`);
  const o = s as { overall: object; byJudge: Record<string, object>; byLanguage: Record<string, object> };
  console.table({ overall: o.overall, ...Object.fromEntries(Object.entries(o.byJudge).map(([k, v]) => [`judge:${k}`, v])), ...Object.fromEntries(Object.entries(o.byLanguage).map(([k, v]) => [`lang:${k}`, v])) });
}
console.log(`\nfull results: ${file}`);
