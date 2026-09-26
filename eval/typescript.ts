/**
 * The TypeScript implementation (src/) behind the interface of eval/implementation.ts, selected with
 * EVAL_IMPL=typescript. A translation runs the same steps as MarkdownTranslator.translateFiles in src/pipeline.ts.
 * Like the binary's -tmFile and -reviewTm, a saved translation memory can be assembled instead, optionally after a
 * review pass. Token usage is metered per run and purpose, as in the binary's report.
 */
import { readFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadConfig } from '../src/config.js';
import { MarkdownTranslator, languageName, sameLanguage } from '../src/pipeline.js';
import { ChatClient, type ChatMessage } from '../src/azure/openai.js';
import { Semaphore } from '../src/azure/http.js';
import { extract } from '../src/markdown/extract.js';
import { assembleDocument } from '../src/assemble.js';
import { renderSegment } from '../src/markdown/render.js';
import { TranslationCache } from '../src/translate/cache.js';
import { reviewPass, translateDocument, type SegmentOutcome, type TranslateTask } from '../src/translate/engine.js';
import { PROMPT_VERSION, type DocAnalysis } from '../src/translate/prompts.js';
import type { Extraction } from '../src/markdown/types.js';
import type { ExtractionDump, LanguageConfig, LanguageReport, Outcome, Report, Switches, TranslateOptions, Usage } from './implementation.js';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')) as { name: string; version: string };
const cfg = loadConfig();
const t = await MarkdownTranslator.create({ ...cfg, cacheDir: undefined });
// All translation and review requests share one limit, like a single translator process.
const limiter = new Semaphore(cfg.maxConcurrency);
const caches = new Map<string, TranslationCache>();
const totals: Record<string, Usage> = {};
const doNotTranslate = [...new Set(t.glossary.doNotTranslate)];

function extractFile(file: string, s: Partial<Switches>): { content: string; ex: Extraction } {
  const content = readFileSync(file, 'utf8');
  const ex = extract(content, doNotTranslate, {
    parse: { mdx: s.mdx ?? /\.mdx$/i.test(file), mathSingleDollar: s.mathSingleDollar ?? cfg.mathSingleDollar },
    docstrings: s.docstrings ?? cfg.docstrings,
    codeComments: s.codeComments ?? cfg.codeComments,
    frontMatter: s.frontMatter ?? cfg.frontMatter,
  });
  return { content, ex };
}

function add(into: Record<string, Usage>, key: string, u: Usage) {
  const x = (into[key] ??= { promptTokens: 0, completionTokens: 0, calls: 0 });
  x.promptTokens += u.promptTokens;
  x.completionTokens += u.completionTokens;
  x.calls += u.calls;
}

/** A chat client whose calls are metered per purpose (the response schema: translations, review). */
function meteredChat() {
  const clients = new Map<string, ChatClient>();
  const chat = Object.create(t.chat) as ChatClient;
  chat.json = <T>(deployment: string, messages: ChatMessage[], schemaName: string, schema: object, reasoningEffort?: string): Promise<T> => {
    let c = clients.get(schemaName);
    if (!c) clients.set(schemaName, (c = new ChatClient(cfg, t.auth, limiter)));
    return c.json<T>(deployment, messages, schemaName, schema, reasoningEffort);
  };
  const usage = () => {
    const byDeployment: Record<string, Usage> = {};
    const byPurpose: Record<string, Usage> = {};
    for (const [purpose, c] of clients) {
      for (const [deployment, u] of c.usage.byDeployment) {
        add(byDeployment, deployment, u);
        add(byPurpose, purpose, u);
      }
    }
    return { byDeployment, byPurpose };
  };
  return { chat, usage };
}

/** One segment cache per directory, shared by all runs that name it (the baseline and variants of a document). */
function cacheFor(dir?: string): TranslationCache {
  if (!dir) return new TranslationCache();
  let c = caches.get(dir);
  if (!c) caches.set(dir, (c = new TranslationCache(dir)));
  return c;
}

export async function version(): Promise<string> {
  return `${pkg.name} ${pkg.version} (TypeScript, prompts ${PROMPT_VERSION})`;
}

export async function languages(): Promise<{ defaultTargets: string[]; languages: Map<string, LanguageConfig> }> {
  return { defaultTargets: t.catalog.defaultTargets, languages: t.catalog.languages };
}

export async function extraction(file: string, s: Partial<Switches>): Promise<ExtractionDump> {
  const { ex } = extractFile(file, s);
  return {
    segments: ex.segments.map((g) => ({ id: g.id, kind: g.kind, textContext: g.textContext, note: g.note, structure: g.structure ?? null, masked: g.masked, original: g.original, passive: g.passive ?? false })),
    notes: ex.notes,
    frontmatterFormality: ex.frontmatterFormality ?? null,
  };
}

export async function analyze(file: string): Promise<unknown> {
  const { ex } = extractFile(file, {});
  return t.analyze(basename(file), ex.source);
}

export function tokens(rep: Report, purpose: 'translations' | 'review' | 'analysis'): number {
  const u = rep.usage?.byPurpose?.[purpose];
  return u ? u.promptTokens + u.completionTokens : 0;
}

export function usage(): Record<string, Usage> {
  // Document analyses run on the translator's own client, translations and reviews on the metered ones.
  const all: Record<string, Usage> = {};
  for (const [k, u] of Object.entries(totals)) add(all, k, u);
  for (const [k, u] of t.chat.usage.byDeployment) add(all, k, u);
  return all;
}

export async function translate(file: string, lang: string, s: Partial<Switches>, o: TranslateOptions): Promise<Report> {
  const started = Date.now();
  const engine = o.engine ?? 'gpt';
  if (engine === 'pseudo') throw new Error('the TypeScript implementation has no pseudo engine');
  const name = basename(file);
  const { content, ex } = extractFile(file, s);
  const [target] = t.resolveLanguages([lang]);
  const translatable = ex.segments.some((g) => !g.passive);
  const metered = meteredChat();
  const analysis = (o.analysis !== undefined ? (o.analysis ?? undefined) : translatable && engine === 'gpt' && !o.memory ? await t.analyze(name, ex.source) : undefined) as DocAnalysis | undefined;
  const sourceCode = (o.sourceLanguage ?? cfg.sourceLanguage ?? analysis?.sourceLanguage ?? 'en').trim();
  const formality = o.formality ?? ex.frontmatterFormality ?? (analysis?.register === 'informal' ? 'informal' : 'formal');
  const report: LanguageReport = { language: target.code, segments: 0, via: {}, retried: 0, reviewEdits: 0, revertedForStructure: [], anchorsAdded: [], keptSource: [], notes: ex.notes, tm: [], outcomes: [] };
  let text = content;
  let tm = new Map<string, string>();
  let outcomes = new Map<string, Outcome>();
  if (!sameLanguage(sourceCode, target.code) && translatable) {
    try {
      const deps = { cfg, chat: metered.chat, nmt: t.nmt, cache: cacheFor(o.cacheDir) };
      const task: TranslateTask = {
        docName: name, ex, lang: target, sourceLanguage: languageName(sourceCode), sourceLanguageCode: sourceCode.split('-')[0], analysis, formality,
        glossary: t.glossary, engine, review: s.review ?? cfg.review, translateDeployment: o.translateDeployment, reviewDeployment: o.reviewDeployment,
        structuralContext: s.structuralContext, nmtFallback: s.nmtFallback,
      };
      if (o.memory) {
        tm = new Map(o.memory.tm);
        outcomes = new Map(o.memory.outcomes.map((x) => [x.id, { ...x }]));
        const remembered = ex.segments.filter((g) => !g.passive && tm.has(g.id));
        for (const g of remembered) if (!outcomes.has(g.id)) outcomes.set(g.id, { id: g.id, kind: g.kind, via: 'tm', retries: 0 });
        if (o.memory.review && metered.chat.available) await reviewPass(deps, task, tm, outcomes as unknown as Map<string, SegmentOutcome>, remembered);
      } else {
        ({ tm, outcomes } = await translateDocument(deps, task));
      }
      const assembled = assembleDocument(ex, tm, { wrap: target.wrap !== 'none', preserveAnchors: s.preserveAnchors ?? cfg.preserveAnchors });
      text = assembled.text;
      report.segments = outcomes.size;
      for (const x of outcomes.values()) {
        report.via[x.via] = (report.via[x.via] ?? 0) + 1;
        if (x.retries) report.retried++;
        if (x.review) report.reviewEdits++;
        if (x.via === 'source') report.keptSource.push({ id: x.id, errors: x.errors ?? [] });
      }
      report.revertedForStructure = assembled.reverted;
      report.anchorsAdded = assembled.anchors;
    } catch (e) {
      // Like the REST API: a file that fails is returned unchanged, with the reason in the report.
      report.error = (e as Error).message;
      text = content;
    }
  }
  const order = new Map(ex.segments.map((g, i) => [g.id, i]));
  report.tm = [...tm].sort(([a], [b]) => (order.get(a) ?? Infinity) - (order.get(b) ?? Infinity));
  report.outcomes = [...outcomes.values()];
  if (o.reportSegments) {
    report.segmentsDetail = ex.segments
      .filter((g) => !g.passive)
      .map((g) => ({ id: g.id, kind: g.kind, note: g.note, masked: g.masked, original: g.original, rendered: renderSegment(g, tm, ex.byId, { wrap: false }) }));
  }
  const usage = metered.usage();
  for (const [k, u] of Object.entries(usage.byDeployment)) add(totals, k, u);
  return { file: name, engine, seconds: (Date.now() - started) / 1000, sourceLanguage: sourceCode, formality, analysis: analysis ?? null, languages: [report], usage, text };
}
