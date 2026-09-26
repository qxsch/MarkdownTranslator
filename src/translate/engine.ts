import type { AppConfig, Glossary, LanguageConfig } from '../config.js';
import type { ChatClient, ChatMessage } from '../azure/openai.js';
import { TruncatedOutputError } from '../azure/openai.js';
import { NmtClient, htmlToMasked, maskedToHtml } from '../azure/nmt.js';
import type { Extraction, Segment } from '../markdown/types.js';
import { TranslationCache } from './cache.js';
import {
  PROMPT_VERSION,
  REVIEW_SCHEMA,
  TRANSLATION_SCHEMA,
  documentContext,
  reviewSystem,
  segmentPayload,
  translationSystem,
  type DocAnalysis,
} from './prompts.js';
import { looksUntranslated, sanitize, validateSegment } from './validate.js';

export type Engine = 'gpt' | 'nmt';

export interface TranslateTask {
  docName: string;
  ex: Extraction;
  lang: LanguageConfig;
  sourceLanguage: string;
  sourceLanguageCode?: string;
  analysis?: DocAnalysis;
  formality: 'formal' | 'informal';
  glossary: Glossary;
  engine: Engine;
  review: boolean;
  translateDeployment?: string;
  reviewDeployment?: string;
  /** Include each segment's structural position in model requests (default: config). */
  structuralContext?: boolean;
  /** Use Azure Translator for segments that fail validation (default: config). */
  nmtFallback?: boolean;
}

export interface SegmentOutcome {
  id: string;
  kind: string;
  via: 'cache' | 'gpt' | 'nmt' | 'source';
  retries: number;
  errors?: string[];
  review?: { category: string; severity: string; explanation: string; before?: string };
  untranslated?: boolean;
}

export interface EngineDeps {
  cfg: AppConfig;
  chat: ChatClient;
  nmt: NmtClient;
  cache: TranslationCache;
}

interface Failure {
  seg: Segment;
  attempt?: string;
  errors: string[];
}

interface TranslationResponse {
  translations: { id: string; text: string }[];
}

interface ReviewResponse {
  edits: { id: string; category: string; severity: string; explanation: string; text: string }[];
}

const MAX_RETRY_ROUNDS = 2;

function batches(segs: Segment[], maxSegs: number, maxChars: number): Segment[][] {
  const out: Segment[][] = [];
  let cur: Segment[] = [];
  let chars = 0;
  for (const s of segs) {
    if (cur.length && (cur.length >= maxSegs || chars + s.masked.length > maxChars)) {
      out.push(cur);
      cur = [];
      chars = 0;
    }
    cur.push(s);
    chars += s.masked.length;
  }
  if (cur.length) out.push(cur);
  return out;
}

export async function translateDocument(deps: EngineDeps, task: TranslateTask): Promise<{ tm: Map<string, string>; outcomes: Map<string, SegmentOutcome> }> {
  const { cfg, chat, nmt, cache } = deps;
  const segs = task.ex.segments.filter((s) => !s.passive);
  const tm = new Map<string, string>();
  const outcomes = new Map<string, SegmentOutcome>();
  const translateDeployment = task.translateDeployment ?? cfg.translateDeployment;
  const reviewDeployment = task.reviewDeployment ?? cfg.reviewDeployment;
  const useGpt = task.engine === 'gpt' && chat.available;
  const review = task.review && useGpt;
  const withStructure = task.structuralContext ?? cfg.structuralContext;
  const glossaryKey = TranslationCache.key([task.glossary, task.analysis?.doNotTranslate ?? [], task.analysis?.terminology ?? []]);
  const index = new Map(segs.map((s, i) => [s.id, i]));
  const keyOf = (seg: Segment) => {
    const i = index.get(seg.id)!;
    return TranslationCache.key([
      PROMPT_VERSION, task.engine, useGpt ? translateDeployment : 'nmt', review ? reviewDeployment : '', task.lang.code, task.formality,
      seg.kind, seg.note, seg.masked, segs[i - 1]?.masked ?? '', segs[i + 1]?.masked ?? '', glossaryKey, withStructure ? seg.structure ?? '' : false,
    ]);
  };

  const pending: Segment[] = [];
  for (const seg of segs) {
    const hit = await cache.get(keyOf(seg));
    if (hit !== undefined && validateSegment(seg, hit, task.ex, task.lang).length === 0) {
      tm.set(seg.id, hit);
      outcomes.set(seg.id, { id: seg.id, kind: seg.kind, via: 'cache', retries: 0 });
    } else pending.push(seg);
  }

  const accept = (seg: Segment, text: string, via: SegmentOutcome['via'], retries: number) => {
    tm.set(seg.id, text);
    outcomes.set(seg.id, { id: seg.id, kind: seg.kind, via, retries, untranslated: looksUntranslated(seg, text) || undefined });
  };

  let failures: Failure[] = [];
  if (useGpt && pending.length) {
    const system = translationSystem({ source: task.sourceLanguage, lang: task.lang, formality: task.formality, analysis: task.analysis, glossary: task.glossary });
    const context = documentContext({ analysis: task.analysis, docName: task.docName, source: task.ex.source, maxChars: cfg.contextMaxChars });

    const callBatch = async (batch: Segment[], retryInfo?: Map<string, Failure>): Promise<Failure[]> => {
      const payload = batch.map((s) => {
        const f = retryInfo?.get(s.id);
        return f ? { ...segmentPayload(s, withStructure), previousAttempt: f.attempt ?? null, problems: f.errors } : segmentPayload(s, withStructure);
      });
      const intro = retryInfo
        ? 'These segments failed automatic validation. Translate them again and fix the listed problems.'
        : 'Translate these segments.';
      const messages: ChatMessage[] = [
        { role: 'system', content: system },
        { role: 'user', content: context },
        { role: 'user', content: `${intro}\n${JSON.stringify({ segments: payload })}` },
      ];
      let res: TranslationResponse;
      try {
        res = await chat.json<TranslationResponse>(translateDeployment, messages, 'translations', TRANSLATION_SCHEMA, cfg.translateReasoning);
      } catch (e) {
        if (e instanceof TruncatedOutputError && batch.length > 1) {
          const mid = Math.ceil(batch.length / 2);
          return [...(await callBatch(batch.slice(0, mid), retryInfo)), ...(await callBatch(batch.slice(mid), retryInfo))];
        }
        return batch.map((seg) => ({ seg, errors: [`model call failed: ${(e as Error).message}`] }));
      }
      const byId = new Map(res.translations.map((t) => [t.id, t.text]));
      const failed: Failure[] = [];
      for (const seg of batch) {
        const raw = byId.get(seg.id);
        if (raw === undefined) {
          failed.push({ seg, errors: ['segment missing from the response'] });
          continue;
        }
        const text = sanitize(seg, raw);
        const errors = validateSegment(seg, text, task.ex, task.lang);
        if (errors.length) failed.push({ seg, attempt: text, errors });
        else accept(seg, text, 'gpt', retryInfo ? 1 : 0);
      }
      return failed;
    };

    const results = await Promise.all(batches(pending, cfg.batchMaxSegments, cfg.batchMaxChars).map((b) => callBatch(b)));
    failures = results.flat();
    for (let round = 1; round <= MAX_RETRY_ROUNDS && failures.length; round++) {
      const info = new Map(failures.map((f) => [f.seg.id, f]));
      const retried = await Promise.all(batches(failures.map((f) => f.seg), 10, cfg.batchMaxChars).map((b) => callBatch(b, info)));
      failures = retried.flat();
      for (const seg of segs) {
        const o = outcomes.get(seg.id);
        if (o && info.has(seg.id)) o.retries = round;
      }
    }
  } else {
    failures = pending.map((seg) => ({ seg, errors: [] }));
  }

  if (failures.length && nmt.available && ((task.nmtFallback ?? cfg.nmtFallback) || task.engine === 'nmt')) {
    try {
      const out = await nmt.translate(
        failures.map((f) => maskedToHtml(f.seg.masked)),
        task.lang.translator ?? task.lang.code,
        task.sourceLanguageCode,
      );
      const remaining: Failure[] = [];
      failures.forEach((f, i) => {
        const text = sanitize(f.seg, htmlToMasked(out[i] ?? ''));
        const errors = validateSegment(f.seg, text, task.ex, task.lang);
        if (errors.length) remaining.push({ ...f, errors: [...f.errors, ...errors.map((e) => `nmt: ${e}`)] });
        else accept(f.seg, text, 'nmt', task.engine === 'nmt' ? 0 : MAX_RETRY_ROUNDS);
      });
      failures = remaining;
    } catch (e) {
      failures = failures.map((f) => ({ ...f, errors: [...f.errors, `nmt failed: ${(e as Error).message}`] }));
    }
  }
  for (const f of failures) outcomes.set(f.seg.id, { id: f.seg.id, kind: f.seg.kind, via: 'source', retries: MAX_RETRY_ROUNDS, errors: f.errors });

  if (review) await reviewPass(deps, task, tm, outcomes, segs.filter((s) => tm.has(s.id) && outcomes.get(s.id)?.via !== 'cache'));

  await Promise.all(
    segs.filter((s) => tm.has(s.id) && outcomes.get(s.id)?.via !== 'cache' && outcomes.get(s.id)?.via !== 'source').map((s) => cache.set(keyOf(s), tm.get(s.id)!)),
  );
  return { tm, outcomes };
}

/** Second pass: a reviewer model checks each translation against its source and returns corrections. */
export async function reviewPass(deps: EngineDeps, task: TranslateTask, tm: Map<string, string>, outcomes: Map<string, SegmentOutcome>, toReview: Segment[]) {
  const { cfg, chat } = deps;
  const reviewDeployment = task.reviewDeployment ?? cfg.reviewDeployment;
  const system = reviewSystem({ source: task.sourceLanguage, lang: task.lang, formality: task.formality, glossary: task.glossary, analysis: task.analysis });
  const context = documentContext({ analysis: task.analysis, docName: task.docName, source: task.ex.source, maxChars: cfg.contextMaxChars });
  await Promise.all(
    batches(toReview, cfg.batchMaxSegments, cfg.batchMaxChars).map(async (batch) => {
      const payload = batch.map((s) => ({ ...segmentPayload(s, task.structuralContext ?? cfg.structuralContext), translation: tm.get(s.id) }));
      let res: ReviewResponse;
      try {
        res = await chat.json<ReviewResponse>(
          reviewDeployment,
          [
            { role: 'system', content: system },
            { role: 'user', content: context },
            { role: 'user', content: `Review these translations.\n${JSON.stringify({ segments: payload })}` },
          ],
          'review',
          REVIEW_SCHEMA,
          cfg.reviewReasoning,
        );
      } catch {
        return;
      }
      const inBatch = new Map(batch.map((s) => [s.id, s]));
      for (const edit of res.edits) {
        const seg = inBatch.get(edit.id);
        if (!seg) continue;
        const text = sanitize(seg, edit.text);
        if (text === tm.get(seg.id) || validateSegment(seg, text, task.ex, task.lang).length) continue;
        const before = tm.get(seg.id);
        tm.set(seg.id, text);
        const o = outcomes.get(seg.id);
        if (!o) continue;
        o.review = { category: edit.category, severity: edit.severity, explanation: edit.explanation, before };
        o.untranslated = looksUntranslated(seg, text) || undefined;
      }
    }),
  );
}
