/**
 * Builds the evaluation report (Markdown + SVG charts + CSV data + regression history) from a features.json run.
 *
 *   npx tsx eval/report.ts                                   # newest eval/results/features-<stamp>/features.json
 *   npx tsx eval/report.ts eval/results/features-<stamp>/features.json
 *   EVAL_REPORT_DIR=evaluation-de-fr npx tsx eval/report.ts  # other output folder
 */
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { forest, groupedBars, heatmap, preference, stackedBars, COLORS } from './charts.js';
import { binomialP, bootstrapReduction, cohenKappa, fleissKappa, holm, majority, quantile, wilson } from './stats.js';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, '..');
const WEIGHT: Record<string, number> = { minor: 1, major: 5, critical: 10 };

type Verdict = 'on' | 'off' | 'tie' | 'inconsistent' | 'error';
interface JudgementRow {
  comparison: string;
  feature: string;
  doc: string;
  lang: string;
  judge: string;
  item: string;
  words: number;
  verdict: Verdict;
  order1?: string;
  order2?: string;
  mqmOn?: number;
  mqmOff?: number;
  errorsOn?: { category: string; severity: string }[];
  errorsOff?: { category: string; severity: string }[];
}
interface SideMetrics {
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
  seconds: number;
  tokens: { translate: number; review: number };
  reused: boolean;
}
interface RecordRow {
  comparison: string;
  feature: string;
  doc: string;
  lang: string;
  defaultOn: boolean;
  changed: boolean;
  on: SideMetrics;
  off: SideMetrics;
}
interface DefectRow {
  doc: string;
  lang: string;
  error?: string;
  keepMissing: string[];
  translateLeft: string[];
  structureBroken: number;
  codeChanged: number;
  linksBroken: number;
  englishByKind: Record<string, number>;
  reverted: number;
  keptSource: number;
  nmt: number;
  tokens: { translate: number; review: number };
  seconds: number;
}
interface Results {
  meta: {
    createdAt: string;
    commit: string;
    langs: string[];
    docs: string[];
    judges: string[];
    judgePanel?: { id: string; provider: string; model: string; family: string }[];
    features: string[];
    config: { translateDeployment: string; reviewDeployment: string; translateReasoning: string; reviewReasoning: string; defaults: Record<string, boolean> };
    /** Version string of the implementation under test (the Rust binary, or the TypeScript pipeline with EVAL_IMPL=typescript); absent in runs from before it was recorded. */
    implementation?: string;
  };
  records: RecordRow[];
  judgements: JudgementRow[];
  defects: DefectRow[];
}

const CATEGORY_RULES: [RegExp, string][] = [
  [/^mixed-/, 'mixed'],
  [/^docstrings-/, 'docstrings'],
  [/^code-/, 'code'],
  [/^frontmatter-/, 'front matter'],
  [/^math-/, 'math'],
  [/\.mdx$|^mdx-/, 'MDX'],
  [/^(anchors-|toc-|links-)/, 'links and anchors'],
  [/^(html-|images-)/, 'HTML and images'],
  [/^(admonitions-|directives|msdocs-|hugo-)/, 'dialects'],
  [/^(emphasis-|escapes-|line-breaks|crlf-|unicode-|tables-|footnotes|task-lists|nested-lists|single-word|inline-code|filenames-|identifiers-|numbers-|product-)/, 'syntax edge cases'],
  [/.*/, 'prose and UI'],
];
export const categoryOf = (doc: string) => CATEGORY_RULES.find(([re]) => re.test(doc))![1];

const CORRECTNESS = ['keepMissing', 'translateLeft', 'structureBroken', 'codeChanged', 'linksBroken', 'englishLeft', 'errors'] as const;
const problems = (m: SideMetrics) => CORRECTNESS.reduce((n, k) => n + m[k], 0);
const sum = <T>(xs: T[], f: (x: T) => number) => xs.reduce((n, x) => n + f(x), 0);
const pct = (v: number, digits = 1) => (Number.isFinite(v) ? `${(v * 100).toFixed(digits)}%` : 'n/a');
const num = (v: number, digits = 2) => (Number.isFinite(v) ? v.toFixed(digits) : 'n/a');
const pFmt = (p: number) => (!Number.isFinite(p) ? 'n/a' : p < 0.001 ? '< 0.001' : p.toFixed(3));
const csv = (rows: Record<string, unknown>[]) => {
  if (!rows.length) return '';
  const cols = Object.keys(rows[0]);
  const cell = (v: unknown) => {
    const s = v === undefined || v === null ? '' : typeof v === 'object' ? JSON.stringify(v) : String(v);
    return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  return [cols.join(','), ...rows.map((r) => cols.map((c) => cell(r[c])).join(','))].join('\n') + '\n';
};
const itemKey = (j: JudgementRow) => `${j.comparison}\u0000${j.doc}\u0000${j.lang}\u0000${j.item}`;

interface FeatureStats {
  comparison: string;
  feature: string;
  shippedDefault: 'on' | 'off';
  documents: number;
  affected: number;
  items: number;
  panel: { wins: number; losses: number; ties: number; p: number; pAdj: number; winRate: number; ci: [number, number] };
  independent: { wins: number; losses: number; ties: number; p: number; judges: string[] };
  mqm: { on: number; off: number; reduction: { estimate: number; lo: number; hi: number } };
  byJudge: Record<string, { wins: number; losses: number; ties: number; inconsistent: number; p: number }>;
  byLang: Record<string, { wins: number; losses: number; reduction: number; items: number }>;
  categoriesOn: Record<string, number>;
  categoriesOff: Record<string, number>;
  on: Record<string, number>;
  off: Record<string, number>;
  englishOn: Record<string, number>;
  englishOff: Record<string, number>;
  problemsOn: number;
  problemsOff: number;
  quality: 'better' | 'worse' | 'no significant difference' | 'not judged';
  correctness: 'better' | 'worse' | 'same';
  recommendation: 'on' | 'off' | 'review manually' | 'not exercised';
  matchesDefault: boolean;
}

function latestResults(): string {
  const dir = join(here, 'results');
  const runs = existsSync(dir)
    ? readdirSync(dir).filter((d) => d.startsWith('features-') && existsSync(join(dir, d, 'features.json'))).map((d) => join(dir, d, 'features.json'))
    : [];
  if (!runs.length) throw new Error('no eval/results/features-*/features.json found; run "npx tsx eval/features.ts" first');
  return runs.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
}

export function buildReport(resultsFile: string, outDir = process.env.EVAL_REPORT_DIR ?? join(repoRoot, 'evaluation')): string {
  const r: Results = JSON.parse(readFileSync(resultsFile, 'utf8'));
  const expectations: Record<string, { keep: string[]; translate: string[] }> = JSON.parse(readFileSync(join(here, 'features', 'expect.json'), 'utf8'));
  const { meta } = r;
  const panelInfo = meta.judgePanel ?? meta.judges.map((id) => ({ id, provider: id.startsWith('copilot:') ? 'copilot' : 'foundry', model: id.replace(/^(copilot|foundry):/, ''), family: id.startsWith('copilot:') && /claude/i.test(id) ? 'anthropic' : 'openai' }));
  const translatorFamily = process.env.EVAL_TRANSLATOR_FAMILY ?? 'openai';
  const independentJudges = panelInfo.filter((j) => j.family !== translatorFamily).map((j) => j.id);
  const judges = panelInfo.map((j) => j.id);
  const images = join(outDir, 'images');
  const data = join(outDir, 'data');
  const history = join(outDir, 'history');
  for (const d of [outDir, images, data, history]) mkdirSync(d, { recursive: true });
  const img = (name: string, svg: string) => {
    writeFileSync(join(images, name), svg);
    return `images/${name}`;
  };

  // ------------------------------------------------------------ per feature
  const comparisons = [...new Set(r.records.map((x) => x.comparison))];
  const valid = r.judgements.filter((j) => j.verdict !== 'error');
  const stats: FeatureStats[] = comparisons.map((c) => {
    const recs = r.records.filter((x) => x.comparison === c);
    const js = valid.filter((j) => j.comparison === c);
    const items = new Map<string, JudgementRow[]>();
    for (const j of js) items.set(itemKey(j), [...(items.get(itemKey(j)) ?? []), j]);
    const vote = (rows: JudgementRow[]) => majority(rows.map((x) => (x.verdict === 'inconsistent' ? 'tie' : x.verdict)));
    const panelVotes = [...items.values()].map(vote);
    const wins = panelVotes.filter((v) => v === 'on').length;
    const losses = panelVotes.filter((v) => v === 'off').length;
    const indVotes = [...items.values()].map((rows) => rows.filter((x) => independentJudges.includes(x.judge))).filter((rows) => rows.length).map(vote);
    const perItem = [...items.values()].map((rows) => ({ words: rows[0].words, on: sum(rows, (x) => x.mqmOn ?? 0) / rows.length, off: sum(rows, (x) => x.mqmOff ?? 0) / rows.length, lang: rows[0].lang }));
    const words = sum(perItem, (x) => x.words) / 100 || 1;
    const catSum = (side: 'errorsOn' | 'errorsOff') => {
      const out: Record<string, number> = {};
      for (const j of js) for (const e of j[side] ?? []) out[e.category] = (out[e.category] ?? 0) + (WEIGHT[e.severity] ?? 1) / 2 / Math.max(1, judges.length);
      for (const k of Object.keys(out)) out[k] /= words;
      return out;
    };
    const agg = (side: 'on' | 'off') => {
      const out: Record<string, number> = {};
      for (const k of [...CORRECTNESS, 'reverted', 'keptSource', 'nmt'] as const) out[k] = sum(recs, (x) => x[side][k]);
      out.tokens = sum(recs, (x) => x[side].tokens.translate + x[side].tokens.review);
      return out;
    };
    const eng = (side: 'on' | 'off') => {
      const out: Record<string, number> = {};
      for (const x of recs) for (const [k, v] of Object.entries(x[side].englishByKind)) out[k] = (out[k] ?? 0) + v;
      return out;
    };
    const byJudge = Object.fromEntries(
      judges.map((jd) => {
        const rows = js.filter((x) => x.judge === jd);
        const w = rows.filter((x) => x.verdict === 'on').length;
        const l = rows.filter((x) => x.verdict === 'off').length;
        return [jd, { wins: w, losses: l, ties: rows.filter((x) => x.verdict === 'tie').length, inconsistent: rows.filter((x) => x.verdict === 'inconsistent').length, p: binomialP(w, w + l) }];
      }),
    );
    const byLang = Object.fromEntries(
      meta.langs.map((l) => {
        const keys = [...items.entries()].filter(([, rows]) => rows[0].lang === l);
        const v = keys.map(([, rows]) => vote(rows));
        const li = perItem.filter((x) => x.lang === l);
        const lw = sum(li, (x) => x.words);
        return [l, { wins: v.filter((x) => x === 'on').length, losses: v.filter((x) => x === 'off').length, items: keys.length, reduction: lw ? ((sum(li, (x) => x.off) - sum(li, (x) => x.on)) / lw) * 100 : NaN }];
      }),
    );
    const on = agg('on');
    const off = agg('off');
    const pOn = sum(recs, (x) => problems(x.on));
    const pOff = sum(recs, (x) => problems(x.off));
    return {
      comparison: c, feature: recs[0].feature, shippedDefault: recs[0].defaultOn ? 'on' : 'off', documents: recs.length, affected: recs.filter((x) => x.changed).length, items: items.size,
      panel: { wins, losses, ties: panelVotes.length - wins - losses, p: binomialP(wins, wins + losses), pAdj: NaN, winRate: wins + losses ? wins / (wins + losses) : NaN, ci: wilson(wins, wins + losses) },
      independent: { wins: indVotes.filter((v) => v === 'on').length, losses: indVotes.filter((v) => v === 'off').length, ties: indVotes.filter((v) => v === 'tie').length, p: binomialP(indVotes.filter((v) => v === 'on').length, indVotes.filter((v) => v !== 'tie').length), judges: independentJudges },
      mqm: { on: sum(perItem, (x) => x.on) / words, off: sum(perItem, (x) => x.off) / words, reduction: bootstrapReduction(perItem) },
      byJudge, byLang, categoriesOn: catSum('errorsOn'), categoriesOff: catSum('errorsOff'), on, off, englishOn: eng('on'), englishOff: eng('off'),
      problemsOn: pOn, problemsOff: pOff, quality: 'not judged', correctness: pOn < pOff ? 'better' : pOn > pOff ? 'worse' : 'same', recommendation: 'off', matchesDefault: false,
    };
  });
  const judged = stats.filter((s) => s.panel.wins + s.panel.losses > 0);
  holm(judged.map((s) => s.panel.p)).forEach((p, i) => (judged[i].panel.pAdj = p));
  for (const s of stats) {
    if (s.panel.wins + s.panel.losses > 0) s.quality = s.panel.pAdj < 0.05 ? (s.panel.wins > s.panel.losses ? 'better' : 'worse') : 'no significant difference';
    else s.quality = s.items ? 'no significant difference' : 'not judged';
    s.recommendation =
      s.quality === 'better' && s.correctness === 'worse'
        ? 'review manually'
        : s.quality === 'better' || (s.correctness === 'better' && s.quality !== 'worse')
          ? 'on'
          : s.quality === 'worse' && s.correctness === 'better'
            ? 'review manually'
            : 'off';
    if (s.affected === 0) s.recommendation = 'not exercised';
    s.matchesDefault = s.recommendation === s.shippedDefault || s.recommendation === 'not exercised';
  }

  // ------------------------------------------------------------ default configuration scorecard
  const d = r.defects;
  const keepTotal = sum(d, (x) => expectations[x.doc]?.keep.length ?? 0);
  const translateTotal = sum(d, (x) => expectations[x.doc]?.translate.length ?? 0);
  const defectCount = (x: DefectRow) => x.keepMissing.length + x.translateLeft.length + x.structureBroken + x.codeChanged + x.linksBroken + (x.error ? 1 : 0);
  const tokensPerDoc = d.map((x) => x.tokens.translate + x.tokens.review).sort((a, b) => a - b);
  const secondsPerDoc = d.map((x) => x.seconds).sort((a, b) => a - b);
  const english: Record<string, number> = {};
  for (const x of d) for (const [k, v] of Object.entries(x.englishByKind)) english[k] = (english[k] ?? 0) + v;
  const scorecard = {
    documents: d.length,
    keepPass: keepTotal ? 1 - sum(d, (x) => x.keepMissing.length) / keepTotal : NaN,
    translatePass: translateTotal ? 1 - sum(d, (x) => x.translateLeft.length) / translateTotal : NaN,
    cleanDocuments: d.filter((x) => defectCount(x) === 0).length / Math.max(1, d.length),
    structureBroken: sum(d, (x) => x.structureBroken),
    codeChanged: sum(d, (x) => x.codeChanged),
    linksBroken: sum(d, (x) => x.linksBroken),
    errors: d.filter((x) => x.error).length,
    reverted: sum(d, (x) => x.reverted),
    keptSource: sum(d, (x) => x.keptSource),
    nmtFallbacks: sum(d, (x) => x.nmt),
    englishSegments: sum(Object.values(english), (v) => v),
    tokensMean: sum(tokensPerDoc, (v) => v) / Math.max(1, tokensPerDoc.length),
    secondsMedian: quantile(secondsPerDoc, 0.5),
    secondsP90: quantile(secondsPerDoc, 0.9),
  };
  const categories = [...new Set(meta.docs.map(categoryOf))];
  const byCategory = categories.map((cat) => {
    const rows = d.filter((x) => categoryOf(x.doc) === cat);
    const kt = sum(rows, (x) => expectations[x.doc]?.keep.length ?? 0);
    const tt = sum(rows, (x) => expectations[x.doc]?.translate.length ?? 0);
    return { category: cat, documents: rows.length, keepPass: kt ? 1 - sum(rows, (x) => x.keepMissing.length) / kt : NaN, translatePass: tt ? 1 - sum(rows, (x) => x.translateLeft.length) / tt : NaN, defects: sum(rows, defectCount) };
  });

  // ------------------------------------------------------------ judge reliability
  const itemsAll = new Map<string, Map<string, string>>();
  for (const j of valid) {
    const m = itemsAll.get(itemKey(j)) ?? new Map<string, string>();
    m.set(j.judge, j.verdict === 'inconsistent' ? 'tie' : j.verdict);
    itemsAll.set(itemKey(j), m);
  }
  const reliability = judges.map((jd) => {
    const rows = valid.filter((x) => x.judge === jd);
    const decidedOrTorn = rows.filter((x) => x.verdict === 'on' || x.verdict === 'off' || x.verdict === 'inconsistent').length;
    const agree: [string, string][] = [];
    for (const m of itemsAll.values()) {
      if (!m.has(jd) || m.size < 2) continue;
      const others = [...m.entries()].filter(([k]) => k !== jd).map(([, v]) => v);
      agree.push([m.get(jd)!, majority(others)]);
    }
    return {
      judge: jd, provider: panelInfo.find((p) => p.id === jd)?.provider ?? '', family: panelInfo.find((p) => p.id === jd)?.family ?? '',
      judgements: rows.length, failed: r.judgements.filter((x) => x.judge === jd && x.verdict === 'error').length,
      decisive: rows.length ? rows.filter((x) => x.verdict === 'on' || x.verdict === 'off').length / rows.length : NaN,
      positionConsistency: decidedOrTorn ? 1 - rows.filter((x) => x.verdict === 'inconsistent').length / decidedOrTorn : NaN,
      agreementWithOthers: agree.length ? agree.filter(([a, b]) => a === b).length / agree.length : NaN,
      kappaWithOthers: cohenKappa(agree),
    };
  });
  const kappaMatrix = judges.map((a) =>
    judges.map((b) => {
      if (a === b) return 1;
      const pairs: [string, string][] = [];
      for (const m of itemsAll.values()) if (m.has(a) && m.has(b)) pairs.push([m.get(a)!, m.get(b)!]);
      return pairs.length ? cohenKappa(pairs) : null;
    }),
  );
  const fleiss = fleissKappa([...itemsAll.values()].filter((m) => m.size === judges.length).map((m) => [...m.values()]));

  // ------------------------------------------------------------ cost
  const costOf = (feature: string, side: 'on' | 'off') => {
    const recs = r.records.filter((x) => x.feature === feature);
    return recs.length ? { translate: sum(recs, (x) => x[side].tokens.translate) / recs.length, review: sum(recs, (x) => x[side].tokens.review) / recs.length } : undefined;
  };
  const costRows: { label: string; translate: number; review: number }[] = [{ label: 'default configuration', translate: sum(d, (x) => x.tokens.translate) / Math.max(1, d.length), review: sum(d, (x) => x.tokens.review) / Math.max(1, d.length) }];
  const noReview = costOf('review', 'off');
  if (noReview) costRows.push({ label: 'without review', ...noReview });
  const noCtx = costOf('structuralContext', 'off');
  if (noCtx) costRows.push({ label: 'without structural context', ...noCtx });

  // ------------------------------------------------------------ regression
  const snapshot = {
    createdAt: meta.createdAt, commit: meta.commit, langs: meta.langs, documents: meta.docs.length, judges,
    scorecard,
    features: Object.fromEntries(stats.map((s) => [s.comparison, { recommendation: s.recommendation, quality: s.quality, wins: s.panel.wins, losses: s.panel.losses, pAdj: s.panel.pAdj, reduction: s.mqm.reduction.estimate, problemsOn: s.problemsOn, problemsOff: s.problemsOff }])),
    defects: d.flatMap((x) => [
      ...x.keepMissing.map((s) => `${x.lang}|${x.doc}|overtranslated|${s}`),
      ...x.translateLeft.map((s) => `${x.lang}|${x.doc}|untranslated|${s}`),
      ...(x.structureBroken ? [`${x.lang}|${x.doc}|structure|`] : []),
      ...(x.codeChanged ? [`${x.lang}|${x.doc}|code|${x.codeChanged}`] : []),
      ...(x.linksBroken ? [`${x.lang}|${x.doc}|links|${x.linksBroken}`] : []),
      ...(x.error ? [`${x.lang}|${x.doc}|error|${x.error.slice(0, 120)}`] : []),
    ]),
  };
  const snapName = `${meta.createdAt.replace(/[:.]/g, '-')}_${meta.commit.replace(/[^\w+-]/g, '')}.json`;
  writeFileSync(join(history, snapName), JSON.stringify(snapshot, null, 2));
  const previous = readdirSync(history)
    .filter((f) => f.endsWith('.json') && f !== snapName)
    .map((f) => JSON.parse(readFileSync(join(history, f), 'utf8')) as typeof snapshot)
    .filter((s) => s.createdAt < meta.createdAt)
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0];

  type Change = { area: string; metric: string; before: string; after: string; status: 'regression' | 'improvement' | 'changed' | 'unchanged' };
  const changes: Change[] = [];
  let newDefects: string[] = [];
  let fixedDefects: string[] = [];
  let retiredDefects: string[] = [];
  if (previous) {
    const rate = (m: 'keepPass' | 'translatePass' | 'cleanDocuments', label: string) => {
      const a = previous.scorecard[m];
      const b = scorecard[m];
      changes.push({ area: 'default configuration', metric: label, before: pct(a), after: pct(b), status: b < a - 0.005 ? 'regression' : b > a + 0.005 ? 'improvement' : 'unchanged' });
    };
    rate('keepPass', 'protected content kept');
    rate('translatePass', 'expected text translated');
    rate('cleanDocuments', 'documents without defects');
    for (const [m, label] of [['structureBroken', 'structure breaks'], ['codeChanged', 'code blocks changed'], ['linksBroken', 'broken links'], ['errors', 'failed documents'], ['reverted', 'reverted blocks'], ['keptSource', 'segments kept in English'], ['englishSegments', 'English segments left']] as const) {
      const a = previous.scorecard[m];
      const b = scorecard[m];
      changes.push({ area: 'default configuration', metric: label, before: String(a), after: String(b), status: b > a ? 'regression' : b < a ? 'improvement' : 'unchanged' });
    }
    const ta = previous.scorecard.tokensMean;
    const tb = scorecard.tokensMean;
    changes.push({ area: 'cost', metric: 'tokens per document', before: ta.toFixed(0), after: tb.toFixed(0), status: tb > ta * 1.1 ? 'regression' : tb < ta * 0.9 ? 'improvement' : 'unchanged' });
    for (const s of stats) {
      const p = previous.features[s.comparison];
      if (!p) continue;
      const qualityRank = { better: 2, 'no significant difference': 1, 'not judged': 1, worse: 0 } as Record<string, number>;
      const qs = qualityRank[s.quality] - qualityRank[p.quality];
      changes.push({ area: s.comparison, metric: 'judged quality', before: `${p.quality} (${p.wins}/${p.losses})`, after: `${s.quality} (${s.panel.wins}/${s.panel.losses})`, status: qs < 0 ? 'regression' : qs > 0 ? 'improvement' : 'unchanged' });
      if (p.recommendation !== s.recommendation) changes.push({ area: s.comparison, metric: 'recommendation', before: p.recommendation, after: s.recommendation, status: 'changed' });
    }
    const before = new Set(previous.defects);
    const after = new Set(snapshot.defects);
    newDefects = snapshot.defects.filter((x) => !before.has(x));
    // A missing defect is only a fix if its expectation still exists; otherwise the expectation was changed.
    const stillExpected = (x: string) => {
      const [, doc, type, ...rest] = x.split('|');
      const detail = rest.join('|');
      if (type === 'overtranslated') return !!expectations[doc]?.keep.includes(detail);
      if (type === 'untranslated') return !!expectations[doc]?.translate.includes(detail);
      return meta.docs.includes(doc);
    };
    const gone = previous.defects.filter((x) => !after.has(x));
    fixedDefects = gone.filter(stillExpected);
    retiredDefects = gone.filter((x) => !stillExpected(x));
  }
  const regressions = changes.filter((c) => c.status === 'regression');

  // ------------------------------------------------------------ charts
  const judgedStats = stats.filter((s) => s.items > 0);
  const charts = {
    forest: img('quality-mqm-reduction.svg', forest({
      title: 'Judged quality: error reduction when the feature is on',
      subtitle: 'MQM penalty points per 100 source words, (off - on), paired bootstrap 95% CI over judged blocks',
      rows: judgedStats.map((s) => ({ label: s.comparison, value: s.mqm.reduction.estimate, lo: s.mqm.reduction.lo, hi: s.mqm.reduction.hi })),
      xLabel: 'MQM points per 100 words removed by the feature', positiveLabel: 'feature helps →', negativeLabel: '← feature hurts',
    })),
    preference: img('quality-preference.svg', preference({
      title: 'Blind pairwise preference (judge panel majority)',
      subtitle: `Blocks that differ between on and off; each judged twice with swapped order by ${judges.length} judge(s)`,
      rows: judgedStats.map((s) => ({ label: s.comparison, wins: s.panel.wins, ties: s.panel.ties, losses: s.panel.losses })),
    })),
    errors: img('quality-error-categories.svg', (() => {
      const cats = [...new Set(judgedStats.flatMap((s) => [...Object.keys(s.categoriesOn), ...Object.keys(s.categoriesOff)]))].sort();
      const rows = judgedStats.flatMap((s) => [`${s.comparison} (on)`, `${s.comparison} (off)`]);
      return stackedBars({
        title: 'Error profile by MQM category', subtitle: 'Penalty points per 100 source words of the judged blocks, averaged over judges',
        rows, segments: cats.map((c) => ({ name: c, values: judgedStats.flatMap((s) => [s.categoriesOn[c] ?? 0, s.categoriesOff[c] ?? 0]) })), unit: 'MQM points per 100 words',
      });
    })()),
    fidelity: img('fidelity-problems.svg', groupedBars({
      title: 'Deterministic problems with the feature on and off',
      subtitle: 'Protected content changed + expected text left in English + structure, code and link breaks + English segments + failures',
      categories: stats.map((s) => s.comparison),
      series: [{ name: 'feature on', values: stats.map((s) => s.problemsOn), color: COLORS.on }, { name: 'feature off', values: stats.map((s) => s.problemsOff), color: COLORS.off }],
      unit: `problems summed over ${meta.langs.length} language(s)`,
    })),
    languages: img('breakdown-languages.svg', heatmap({
      title: 'Error reduction by language', subtitle: 'MQM points per 100 words removed by the feature (off - on); green = feature helps',
      rows: judgedStats.map((s) => s.comparison), cols: meta.langs, values: judgedStats.map((s) => meta.langs.map((l) => (s.byLang[l]?.items ? s.byLang[l].reduction : null))),
    })),
    categories: img('breakdown-document-types.svg', heatmap({
      title: 'Problems avoided by document type', subtitle: 'Deterministic problems with the feature off minus on; green = feature avoids problems',
      rows: stats.map((s) => s.comparison), cols: categories,
      values: stats.map((s) => categories.map((cat) => {
        const recs = r.records.filter((x) => x.comparison === s.comparison && categoryOf(x.doc) === cat);
        return recs.length ? sum(recs, (x) => problems(x.off) - problems(x.on)) : null;
      })),
      format: (v) => (v > 0 ? `+${v}` : String(v)),
    })),
    scorecard: img('default-config-by-document-type.svg', groupedBars({
      title: 'Default configuration: expectation pass rate by document type', subtitle: 'Share of protected strings kept verbatim and of expected phrases translated',
      categories: byCategory.map((c) => c.category),
      series: [{ name: 'protected content kept (%)', values: byCategory.map((c) => +(c.keepPass * 100).toFixed(1) || 0), color: COLORS.on }, { name: 'expected text translated (%)', values: byCategory.map((c) => +(c.translatePass * 100).toFixed(1) || 0), color: COLORS.win }],
      unit: 'percent',
    })),
    judges: img('judge-agreement.svg', heatmap({
      title: 'Judge agreement (Cohen\u2019s kappa, verdict on / off / tie)', subtitle: `Fleiss\u2019 kappa over blocks rated by all judges: ${num(fleiss)}`,
      rows: judges, cols: judges, values: kappaMatrix, format: (v) => v.toFixed(2),
    })),
    cost: img('cost-tokens.svg', groupedBars({
      title: 'Model tokens per document', subtitle: 'Prompt + completion tokens of the translator (Foundry), mean per document and language',
      categories: costRows.map((c) => c.label),
      series: [{ name: 'translation', values: costRows.map((c) => Math.round(c.translate)), color: COLORS.on }, { name: 'review', values: costRows.map((c) => Math.round(c.review)), color: COLORS.off }],
      unit: 'tokens',
    })),
  };

  // ------------------------------------------------------------ data files
  rmSync(join(data, 'results.json'), { force: true });
  // The default-configuration outputs are the evidence people want to read; variants stay in eval/results.
  const translations = join(outDir, 'translations');
  rmSync(translations, { recursive: true, force: true });
  for (const lang of meta.langs) {
    const src = join(dirname(resultsFile), lang, 'baseline');
    if (!existsSync(src)) continue;
    mkdirSync(join(translations, lang), { recursive: true });
    for (const f of readdirSync(src).filter((x) => /\.mdx?$/.test(x))) copyFileSync(join(src, f), join(translations, lang, f));
  }
  writeFileSync(join(data, 'feature-summary.json'), JSON.stringify({ meta, scorecard, byCategory, stats, reliability, fleissKappa: fleiss, regression: { previous: previous ? { createdAt: previous.createdAt, commit: previous.commit } : null, changes, newDefects, fixedDefects, retiredDefects } }, null, 2));
  writeFileSync(join(data, 'judgements.csv'), csv(r.judgements.map((j) => ({ comparison: j.comparison, doc: j.doc, category: categoryOf(j.doc), lang: j.lang, judge: j.judge, item: j.item, words: j.words, order1: j.order1, order2: j.order2, verdict: j.verdict, mqmOn: j.mqmOn, mqmOff: j.mqmOff }))));
  writeFileSync(join(data, 'records.csv'), csv(r.records.map((x) => ({
    comparison: x.comparison, doc: x.doc, category: categoryOf(x.doc), lang: x.lang, defaultOn: x.defaultOn, changed: x.changed,
    ...Object.fromEntries(CORRECTNESS.flatMap((k) => [[`${k}On`, x.on[k]], [`${k}Off`, x.off[k]]])),
    tokensOn: x.on.tokens.translate + x.on.tokens.review, tokensOff: x.off.tokens.translate + x.off.tokens.review,
  }))));
  writeFileSync(join(data, 'defects.csv'), csv(d.flatMap((x) => [
    ...x.keepMissing.map((s) => ({ lang: x.lang, doc: x.doc, category: categoryOf(x.doc), type: 'protected content changed', detail: s })),
    ...x.translateLeft.map((s) => ({ lang: x.lang, doc: x.doc, category: categoryOf(x.doc), type: 'expected text not translated', detail: s })),
    ...(x.structureBroken ? [{ lang: x.lang, doc: x.doc, category: categoryOf(x.doc), type: 'structure changed', detail: '' }] : []),
    ...(x.codeChanged ? [{ lang: x.lang, doc: x.doc, category: categoryOf(x.doc), type: 'code block changed', detail: String(x.codeChanged) }] : []),
    ...(x.linksBroken ? [{ lang: x.lang, doc: x.doc, category: categoryOf(x.doc), type: 'broken in-page links', detail: String(x.linksBroken) }] : []),
    ...(x.error ? [{ lang: x.lang, doc: x.doc, category: categoryOf(x.doc), type: 'translation failed', detail: x.error }] : []),
  ])));

  // ------------------------------------------------------------ markdown
  const L: string[] = [];
  const langList = meta.langs.join(', ');
  const status = !previous ? 'first run with this output folder, no regression baseline yet' : regressions.length ? `${regressions.length} regression(s) against ${previous.commit} (${previous.createdAt.slice(0, 10)})` : `no regressions against ${previous.commit} (${previous.createdAt.slice(0, 10)})`;
  L.push('# Translation quality evaluation', '');
  L.push(`Run ${meta.createdAt.slice(0, 16).replace('T', ' ')} UTC, commit \`${meta.commit}\`. ${meta.docs.length} English documents translated into ${langList}${meta.implementation ? ` by \`${meta.implementation}\`` : ''}. Translator: Foundry deployment \`${meta.config.translateDeployment}\` (reasoning ${meta.config.translateReasoning}), review \`${meta.config.reviewDeployment}\` (reasoning ${meta.config.reviewReasoning}). Judges: ${panelInfo.map((j) => `\`${j.id}\` (${j.provider}, ${j.family})`).join(', ')}.`, '');
  L.push('## Summary', '');
  L.push(`- Regression status: **${status}**.`);
  L.push(`- Default configuration: ${pct(scorecard.keepPass)} of protected strings kept verbatim, ${pct(scorecard.translatePass)} of expected phrases translated, ${pct(scorecard.cleanDocuments)} of documents without any defect, ${scorecard.structureBroken} structure breaks, ${scorecard.codeChanged} changed code blocks, ${scorecard.errors} failed documents.`);
  const mismatches = stats.filter((s) => !s.matchesDefault);
  L.push(mismatches.length ? `- Evidence differs from the shipped default for: ${mismatches.map((s) => `**${s.comparison}** (shipped ${s.shippedDefault}, evidence says ${s.recommendation})`).join('; ')}.` : '- Every shipped default matches the evidence of this run.');
  for (const s of stats.filter((x) => x.quality === 'better' || x.quality === 'worse')) {
    L.push(`- **${s.comparison}** is significantly ${s.quality} with the feature on: won ${s.panel.wins}, lost ${s.panel.losses} (Holm-adjusted p ${pFmt(s.panel.pAdj)}), error score ${num(s.mqm.on)} vs ${num(s.mqm.off)} per 100 words.`);
  }
  L.push('');
  L.push('## Feature decisions', '');
  L.push('Rule: a feature is recommended on when it wins the judged comparison significantly (Holm-adjusted sign test, p < 0.05) or avoids deterministic problems without a significant quality loss; otherwise off. Opposing signals are flagged for manual review. "Not exercised" means the switch changed no output in this corpus, so there is no evidence either way.', '');
  L.push('| Feature | Shipped | Evidence | Affected docs | Won / tie / lost | Win rate (95% CI) | p (Holm) | MQM on / off | Reduction (95% CI) | Problems on / off |');
  L.push('|---|---|---|---|---|---|---|---|---|---|');
  for (const s of stats) {
    const w = s.panel;
    L.push(`| ${s.comparison} | ${s.shippedDefault} | **${s.recommendation}**${s.matchesDefault ? '' : ' (differs)'} | ${s.affected}/${s.documents} | ${w.wins} / ${w.ties} / ${w.losses} | ${w.wins + w.losses ? `${pct(w.winRate, 0)} (${pct(w.ci[0], 0)} to ${pct(w.ci[1], 0)})` : 'n/a'} | ${pFmt(w.pAdj)} | ${s.items ? `${num(s.mqm.on)} / ${num(s.mqm.off)}` : 'n/a'} | ${s.items ? `${num(s.mqm.reduction.estimate)} (${num(s.mqm.reduction.lo)} to ${num(s.mqm.reduction.hi)})` : 'n/a'} | ${s.problemsOn} / ${s.problemsOff} |`);
  }
  L.push('');
  L.push('## 1. Judged translation quality', '');
  L.push(`Every block (paragraph, list, table, code block, front matter) whose output differs between the feature on and off is judged blind: each judge sees the English source and both candidates as A and B, twice with the order swapped. A judge's verdict counts only when both orders agree; the panel verdict is the majority of the judges. Error annotations follow MQM (minor 1, major 5, critical 10 points), normalised per 100 source words.`, '');
  L.push(`![Error reduction with confidence intervals](${charts.forest})`, '');
  L.push(`![Pairwise preference](${charts.preference})`, '');
  if (independentJudges.length) {
    L.push(`Judges from a different model family than the translator (${independentJudges.map((j) => `\`${j}\``).join(', ')}) guard against self-preference:`, '');
    L.push('| Feature | Independent judges: won / tie / lost | p (unadjusted) |', '|---|---|---|');
    for (const s of judgedStats) L.push(`| ${s.comparison} | ${s.independent.wins} / ${s.independent.ties} / ${s.independent.losses} | ${pFmt(s.independent.p)} |`);
    L.push('');
  }
  L.push('Per judge (won / lost with the feature on, sign test):', '');
  L.push(`| Feature | ${judges.map((j) => `\`${j}\``).join(' | ')} |`, `|---|${judges.map(() => '---').join('|')}|`);
  for (const s of judgedStats) L.push(`| ${s.comparison} | ${judges.map((j) => `${s.byJudge[j].wins} / ${s.byJudge[j].losses} (p ${pFmt(s.byJudge[j].p)})`).join(' | ')} |`);
  L.push('');
  L.push('## 2. Error profile', '');
  L.push(`![Error categories](${charts.errors})`, '');
  L.push('## 3. Fidelity and correctness (deterministic)', '');
  L.push('Checked on every output without a model: expectations in [eval/features/expect.json](../eval/features/expect.json) (strings that must stay byte-identical, phrases that must be translated), Markdown structure re-parsed and compared node by node, code blocks compared after stripping comments and docstrings, in-page links resolved against the translated headings and anchors, and source segments that come back unchanged (English left behind).', '');
  L.push(`![Deterministic problems](${charts.fidelity})`, '');
  L.push('| Feature | Protected changed on / off | Not translated on / off | Structure on / off | Code on / off | Links on / off | English segments on / off | Failures on / off | Tokens on / off |');
  L.push('|---|---|---|---|---|---|---|---|---|');
  for (const s of stats) L.push(`| ${s.comparison} | ${s.on.keepMissing} / ${s.off.keepMissing} | ${s.on.translateLeft} / ${s.off.translateLeft} | ${s.on.structureBroken} / ${s.off.structureBroken} | ${s.on.codeChanged} / ${s.off.codeChanged} | ${s.on.linksBroken} / ${s.off.linksBroken} | ${s.on.englishLeft} / ${s.off.englishLeft} | ${s.on.errors} / ${s.off.errors} | ${s.on.tokens} / ${s.off.tokens} |`);
  L.push('');
  L.push(`![Problems avoided by document type](${charts.categories})`, '');
  L.push('## 4. Default configuration scorecard', '');
  L.push('| Metric | Value |', '|---|---|');
  L.push(`| Documents x languages | ${scorecard.documents} |`, `| Protected strings kept verbatim | ${pct(scorecard.keepPass)} of ${keepTotal} |`, `| Expected phrases translated | ${pct(scorecard.translatePass)} of ${translateTotal} |`, `| Documents without defects | ${pct(scorecard.cleanDocuments)} |`);
  L.push(`| Structure breaks / changed code blocks / broken links | ${scorecard.structureBroken} / ${scorecard.codeChanged} / ${scorecard.linksBroken} |`, `| Failed documents | ${scorecard.errors} |`, `| Blocks reverted by the structure check | ${scorecard.reverted} |`, `| Segments kept in English after validation | ${scorecard.keptSource} |`, `| Segments translated by the NMT fallback | ${scorecard.nmtFallbacks} |`);
  L.push(`| English segments left (${Object.entries(english).map(([k, v]) => `${k} ${v}`).join(', ') || 'none'}) | ${scorecard.englishSegments} |`, `| Tokens per document (mean) | ${scorecard.tokensMean.toFixed(0)} |`, `| Seconds per document (median / p90) | ${num(scorecard.secondsMedian, 1)} / ${num(scorecard.secondsP90, 1)} |`, '');
  L.push(`![Pass rate by document type](${charts.scorecard})`, '');
  const defectRows = d.filter((x) => defectCount(x) > 0);
  if (defectRows.length) {
    L.push('Defects of the default configuration (full list in [data/defects.csv](data/defects.csv)):', '');
    L.push('| Language | Document | Protected content changed | Expected text not translated | Other |', '|---|---|---|---|---|');
    for (const x of defectRows.slice(0, 40)) {
      const other = [x.structureBroken ? 'structure' : '', x.codeChanged ? `code x${x.codeChanged}` : '', x.linksBroken ? `links x${x.linksBroken}` : '', x.error ? 'failed' : ''].filter(Boolean).join(', ');
      const list = (xs: string[]) => xs.map((s) => `\`${s.replace(/\n/g, '\u21b5').replace(/\|/g, '\\|').replace(/`/g, "'").slice(0, 60)}\``).join(', ');
      L.push(`| ${x.lang} | ${x.doc} | ${list(x.keepMissing)} | ${list(x.translateLeft)} | ${other} |`);
    }
    if (defectRows.length > 40) L.push('', `... and ${defectRows.length - 40} more.`);
    L.push('');
  }
  L.push('## 5. Breakdown by language', '');
  L.push(`![By language](${charts.languages})`, '');
  L.push('## 6. Judge reliability', '');
  L.push('| Judge | Provider | Family | Judgements | Failed | Decisive | Position consistency | Agreement with other judges | Cohen\u2019s kappa vs others |', '|---|---|---|---|---|---|---|---|---|');
  for (const x of reliability) L.push(`| \`${x.judge}\` | ${x.provider} | ${x.family} | ${x.judgements} | ${x.failed} | ${pct(x.decisive, 0)} | ${pct(x.positionConsistency, 0)} | ${pct(x.agreementWithOthers, 0)} | ${num(x.kappaWithOthers)} |`);
  L.push('', `Position consistency is the share of decisive judgements where both presentation orders picked the same candidate. Fleiss\u2019 kappa over all judges: ${num(fleiss)} (0.2 to 0.4 fair, 0.4 to 0.6 moderate, above 0.6 substantial).`, '');
  if (judges.length > 1) L.push(`![Judge agreement](${charts.judges})`, '');
  L.push('## 7. Cost', '');
  L.push(`![Tokens per document](${charts.cost})`, '');
  L.push('## 8. Regression report', '');
  if (!previous) L.push('No earlier run in this folder. This run is the baseline for the next comparison.', '');
  else {
    L.push(`Compared with run ${previous.createdAt.slice(0, 16).replace('T', ' ')} UTC, commit \`${previous.commit}\` (${previous.langs.join(', ')}, ${previous.documents} documents${previous.langs.join() !== meta.langs.join() || previous.documents !== meta.docs.length ? '; different setup, compare with care' : ''}).`, '');
    L.push('| Area | Metric | Before | After | Status |', '|---|---|---|---|---|');
    for (const c of changes.filter((x) => x.status !== 'unchanged')) L.push(`| ${c.area} | ${c.metric} | ${c.before} | ${c.after} | ${c.status === 'regression' ? '**regression**' : c.status} |`);
    if (!changes.some((x) => x.status !== 'unchanged')) L.push('| all | all tracked metrics | | | unchanged |');
    L.push('', `New defects: ${newDefects.length}. Fixed defects: ${fixedDefects.length}. No longer checked because the expectation was changed: ${retiredDefects.length}.`, '');
    for (const [title, list] of [['New defects', newDefects], ['Fixed defects', fixedDefects], ['No longer checked (expectation changed)', retiredDefects]] as const) {
      if (!list.length) continue;
      L.push(`${title}:`, '');
      for (const x of list.slice(0, 30)) {
        const [lang, doc, type, detail] = x.split('|');
        L.push(`- ${lang} ${doc}: ${type}${detail ? ` \`${detail.replace(/\n/g, '\u21b5').replace(/`/g, "'").slice(0, 80)}\`` : ''}`);
      }
      L.push('');
    }
  }
  L.push('## Method', '');
  L.push('Dimensions measured:', '');
  L.push('1. **Judged quality**: blind pairwise preference with position swap, panel majority, exact sign test with Holm correction over all features, Wilson 95% interval of the win rate, MQM error score with a paired bootstrap interval (2000 resamples over judged blocks, fixed seed).');
  L.push('2. **Error profile**: MQM categories (mistranslation, omission, terminology, fluency, register, untranslated, overtranslation, markup, ...) weighted by severity.');
  L.push('3. **Fidelity**: deterministic checks that need no model: protected strings, expected translations, Markdown structure, code bytes, in-page links, English left behind, failures.');
  L.push('4. **Robustness**: validation retries, blocks reverted by the structure check, segments kept in English, NMT fallbacks.');
  L.push('5. **Cost**: tokens and wall-clock time per document.');
  L.push('6. **Judge reliability**: position consistency, pairwise Cohen\u2019s kappa, Fleiss\u2019 kappa, per-judge results and a subset of judges from another model family than the translator.');
  L.push('7. **Regression**: every run stores a snapshot in `history/`; the report compares with the previous snapshot and lists new and fixed defects.', '');
  L.push('Isolation: each feature is flipped alone against the shipped defaults. Segments that the switch does not change are reused from the default run, so differences come from the feature and not from sampling noise. Review off, NMT fallback off and anchors off are derived exactly from the default run.', '');
  L.push('Limitations: LLM judges instead of professional linguists; judge families overlap with the translator unless independent judges are configured; the corpus is synthetic technical documentation; "affected" features with few differing blocks have wide intervals.', '');
  L.push('## Reproduce', '');
  L.push('```powershell');
  L.push('npm install; npm install --prefix eval             # evaluator dependencies (host only, not in the container)');
  L.push(`$env:EVAL_LANGS = '${langList.replace(/, /g, ',')}'`);
  L.push(`$env:EVAL_JUDGES = '${judges.join(',')}'`);
  L.push('npx tsx eval/features.ts                            # translate, check, judge, then write this report');
  L.push('npx tsx eval/report.ts                              # rebuild the report from the latest results');
  L.push('```', '');
  L.push('See the [README](../README.md#evaluation-and-tests) for all options.', '');
  L.push('## Data', '');
  L.push('- [data/feature-summary.json](data/feature-summary.json): every number in this report', '- [data/judgements.csv](data/judgements.csv): one row per judged block and judge', '- [data/records.csv](data/records.csv): deterministic checks per document, language and feature', '- [data/defects.csv](data/defects.csv): defects of the default configuration', `- [translations/](translations/): the translated corpus (default configuration) next to the sources in [eval/features/](../eval/features/)`, '- [history/](history/): snapshots for regression tracking', '');
  writeFileSync(join(outDir, 'README.md'), L.join('\n'));
  return outDir;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const file = process.argv[2] ?? latestResults();
  const dir = buildReport(file);
  console.log(`evaluation report: ${join(dir, 'README.md')}`);
}
