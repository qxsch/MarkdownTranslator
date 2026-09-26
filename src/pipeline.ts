import { loadConfig, loadGlossary, loadLanguages, type AppConfig, type Glossary, type LanguageCatalog, type LanguageConfig } from './config.js';
import { AzureAuth, Semaphore } from './azure/http.js';
import { ChatClient } from './azure/openai.js';
import { NmtClient } from './azure/nmt.js';
import { initCodeParsers } from './code/parsers.js';
import { extract } from './markdown/extract.js';
import { assembleDocument } from './assemble.js';
import type { Extraction } from './markdown/types.js';
import { TranslationCache } from './translate/cache.js';
import { translateDocument, type Engine, type SegmentOutcome } from './translate/engine.js';
import { ANALYSIS_SCHEMA, ANALYSIS_SYSTEM, type DocAnalysis } from './translate/prompts.js';

export interface TranslateOptions {
  structuralContext?: boolean;
  docstrings?: boolean;
  formality?: 'formal' | 'informal';
  sourceLanguage?: string;
  review?: boolean;
  doNotTranslate?: string[];
  engine?: Engine;
  translateDeployment?: string;
  reviewDeployment?: string;
}

export interface FileReport {
  file: string;
  language: string;
  formality: 'formal' | 'informal';
  segments: number;
  via: Record<string, number>;
  retried: number;
  reviewEdits: number;
  untranslatedWarnings: string[];
  keptSource: { id: string; errors: string[] }[];
  revertedForStructure: string[];
  anchorsAdded: string[];
  notes: string[];
  error?: string;
}

export interface TranslateResult {
  translations: Record<string, Record<string, string>>;
  reports: FileReport[];
  analyses: Record<string, DocAnalysis | undefined>;
  outcomes: Record<string, Record<string, Map<string, SegmentOutcome>>>;
  extractions: Record<string, Extraction>;
  usage: unknown;
}

const displayNames = new Intl.DisplayNames(['en'], { type: 'language' });

export class MarkdownTranslator {
  readonly cfg: AppConfig;
  readonly catalog: LanguageCatalog;
  readonly glossary: Glossary;
  readonly chat: ChatClient;
  readonly nmt: NmtClient;
  readonly cache: TranslationCache;
  readonly auth: AzureAuth;

  private constructor(cfg: AppConfig) {
    this.cfg = cfg;
    this.catalog = loadLanguages(cfg.languagesFile);
    this.glossary = loadGlossary(cfg.glossaryFile);
    this.auth = new AzureAuth(cfg);
    const limiter = new Semaphore(cfg.maxConcurrency);
    this.chat = new ChatClient(cfg, this.auth, limiter);
    this.nmt = new NmtClient(cfg, this.auth, limiter);
    this.cache = new TranslationCache(cfg.cacheDir);
  }

  static async create(cfg: AppConfig = loadConfig()): Promise<MarkdownTranslator> {
    await initCodeParsers();
    return new MarkdownTranslator(cfg);
  }

  resolveLanguages(codes?: string[]): LanguageConfig[] {
    const list = codes?.length ? codes : this.catalog.defaultTargets;
    return list.map((c) => {
      const l = this.catalog.languages.get(c.trim().toLowerCase());
      if (!l) throw new UnknownLanguageError(c);
      return l;
    });
  }

  async analyze(docName: string, source: string): Promise<DocAnalysis | undefined> {
    if (!this.chat.available) return undefined;
    const doc = source.length > this.cfg.contextMaxChars ? source.slice(0, this.cfg.contextMaxChars) : source;
    try {
      return await this.chat.json<DocAnalysis>(
        this.cfg.analysisDeployment,
        [
          { role: 'system', content: ANALYSIS_SYSTEM },
          { role: 'user', content: `Document "${docName}":\n<document>\n${doc}\n</document>` },
        ],
        'analysis',
        ANALYSIS_SCHEMA,
        'low',
      );
    } catch {
      return undefined;
    }
  }

  async translateFiles(files: Record<string, string>, targetCodes: string[] | undefined, opts: TranslateOptions = {}): Promise<TranslateResult> {
    const targets = this.resolveLanguages(targetCodes);
    const dnt = [...new Set([...this.glossary.doNotTranslate, ...(opts.doNotTranslate ?? [])])];
    const result: TranslateResult = { translations: {}, reports: [], analyses: {}, outcomes: {}, extractions: {}, usage: undefined };
    for (const t of targets) result.translations[t.code] = {};

    await Promise.all(
      Object.entries(files).map(async ([file, content]) => {
        const ex = extract(content, dnt, {
          parse: { mdx: /\.mdx$/i.test(file), mathSingleDollar: this.cfg.mathSingleDollar },
          docstrings: opts.docstrings ?? this.cfg.docstrings,
        });
        result.extractions[file] = ex;
        const translatable = ex.segments.some((s) => !s.passive);
        const analysis = translatable && (opts.engine ?? 'gpt') === 'gpt' ? await this.analyze(file, ex.source) : undefined;
        result.analyses[file] = analysis;
        result.outcomes[file] = {};
        const sourceCode = (opts.sourceLanguage ?? this.cfg.sourceLanguage ?? analysis?.sourceLanguage ?? 'en').trim();
        const formality = opts.formality ?? ex.frontmatterFormality ?? (analysis?.register === 'informal' ? 'informal' : 'formal');
        await Promise.all(
          targets.map(async (lang) => {
            const report: FileReport = {
              file, language: lang.code, formality, segments: 0, via: {}, retried: 0, reviewEdits: 0, untranslatedWarnings: [], keptSource: [], revertedForStructure: [], anchorsAdded: [], notes: ex.notes,
            };
            result.reports.push(report);
            if (sameLanguage(sourceCode, lang.code) || !translatable) {
              result.translations[lang.code][file] = content;
              return;
            }
            try {
              const { tm, outcomes } = await translateDocument(
                { cfg: this.cfg, chat: this.chat, nmt: this.nmt, cache: this.cache },
                {
                  docName: file, ex, lang, sourceLanguage: languageName(sourceCode), sourceLanguageCode: sourceCode.split('-')[0], analysis, formality,
                  glossary: this.glossary, engine: opts.engine ?? 'gpt', review: opts.review ?? this.cfg.review,
                  translateDeployment: opts.translateDeployment, reviewDeployment: opts.reviewDeployment, structuralContext: opts.structuralContext,
                },
              );
              result.outcomes[file][lang.code] = outcomes;
              const assembled = assembleDocument(ex, tm, { wrap: lang.wrap !== 'none', preserveAnchors: this.cfg.preserveAnchors });
              result.translations[lang.code][file] = assembled.text;
              report.segments = outcomes.size;
              for (const o of outcomes.values()) {
                report.via[o.via] = (report.via[o.via] ?? 0) + 1;
                if (o.retries) report.retried++;
                if (o.review) report.reviewEdits++;
                if (o.untranslated) report.untranslatedWarnings.push(o.id);
                if (o.via === 'source') report.keptSource.push({ id: o.id, errors: o.errors ?? [] });
              }
              report.revertedForStructure = assembled.reverted;
              report.anchorsAdded = assembled.anchors;
            } catch (e) {
              report.error = (e as Error).message;
              result.translations[lang.code][file] = content;
            }
          }),
        );
      }),
    );
    result.usage = this.chat.usage.toJSON();
    return result;
  }
}

export class UnknownLanguageError extends Error {
  constructor(readonly code: string) {
    super(`unknown target language "${code}"`);
  }
}

function sameLanguage(a: string, b: string): boolean {
  const base = (x: string) => x.toLowerCase().split(/[-_]/)[0];
  return base(a) === base(b);
}

function languageName(code: string): string {
  try {
    return displayNames.of(code) ?? code;
  } catch {
    return code;
  }
}
