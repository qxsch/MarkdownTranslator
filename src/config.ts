import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', ...(import.meta.url.includes('/dist/') ? ['..'] : []));

function env(name: string): string | undefined {
  const v = process.env[name]?.trim();
  return v ? v : undefined;
}

function bool(name: string, def: boolean): boolean {
  const v = env(name);
  return v === undefined ? def : /^(1|true|yes|on)$/i.test(v);
}

function int(name: string, def: number): number {
  const v = env(name);
  const n = v === undefined ? NaN : Number.parseInt(v, 10);
  return Number.isFinite(n) && n > 0 ? n : def;
}

const trimSlash = (u: string | undefined) => u?.replace(/\/+$/, '');

export interface AppConfig {
  openaiEndpoint?: string;
  translatorEndpoint?: string;
  translatorRegion?: string;
  /** When set, API-key auth is used for all Azure AI calls; otherwise Microsoft Entra ID (managed identity / developer login). */
  apiKey?: string;
  managedIdentityClientId?: string;
  tenantId?: string;
  /** Pre-acquired Entra access token (local container tests only; expires after about an hour). */
  staticAccessToken?: string;
  translateDeployment: string;
  reviewDeployment: string;
  analysisDeployment: string;
  translateReasoning: string;
  reviewReasoning: string;
  review: boolean;
  nmtFallback: boolean;
  maxConcurrency: number;
  batchMaxSegments: number;
  batchMaxChars: number;
  contextMaxChars: number;
  requestTimeoutMs: number;
  cacheDir?: string;
  apiAuthKey?: string;
  port: number;
  bodyLimitBytes: number;
  languagesFile: string;
  glossaryFile: string;
  preserveAnchors: boolean;
  sourceLanguage?: string;
  /** Send each segment's position (section, table column/row, list lead-in) to the model. */
  structuralContext: boolean;
  mathSingleDollar: boolean;
  docstrings: boolean;
  codeComments: boolean;
  frontMatter: boolean;
}

export function loadConfig(): AppConfig {
  const translateDeployment = env('MDT_TRANSLATE_DEPLOYMENT') ?? 'translate';
  return {
    openaiEndpoint: trimSlash(env('AZURE_OPENAI_ENDPOINT')),
    translatorEndpoint: trimSlash(env('AZURE_TRANSLATOR_ENDPOINT')),
    translatorRegion: env('AZURE_TRANSLATOR_REGION'),
    apiKey: env('AZURE_AI_API_KEY'),
    managedIdentityClientId: env('AZURE_CLIENT_ID'),
    tenantId: env('AZURE_TENANT_ID'),
    staticAccessToken: env('AZURE_AI_ACCESS_TOKEN'),
    translateDeployment,
    reviewDeployment: env('MDT_REVIEW_DEPLOYMENT') ?? translateDeployment,
    analysisDeployment: env('MDT_ANALYSIS_DEPLOYMENT') ?? translateDeployment,
    translateReasoning: env('MDT_TRANSLATE_REASONING') ?? 'medium',
    reviewReasoning: env('MDT_REVIEW_REASONING') ?? 'high',
    review: bool('MDT_REVIEW', true),
    nmtFallback: bool('MDT_NMT_FALLBACK', true),
    maxConcurrency: int('MDT_MAX_CONCURRENCY', 16),
    batchMaxSegments: int('MDT_BATCH_MAX_SEGMENTS', 40),
    batchMaxChars: int('MDT_BATCH_MAX_CHARS', 12000),
    contextMaxChars: int('MDT_CONTEXT_MAX_CHARS', 60000),
    requestTimeoutMs: int('MDT_REQUEST_TIMEOUT_MS', 600000),
    cacheDir: env('MDT_CACHE_DIR'),
    apiAuthKey: env('MDT_API_KEY'),
    port: int('PORT', 8080),
    bodyLimitBytes: int('MDT_BODY_LIMIT_BYTES', 25 * 1024 * 1024),
    languagesFile: env('MDT_LANGUAGES_FILE') ?? join(root, 'config', 'languages.json'),
    glossaryFile: env('MDT_GLOSSARY_FILE') ?? join(root, 'config', 'glossary.json'),
    preserveAnchors: bool('MDT_PRESERVE_ANCHORS', true),
    sourceLanguage: env('MDT_SOURCE_LANGUAGE'),
    structuralContext: bool('MDT_STRUCTURAL_CONTEXT', false),
    mathSingleDollar: bool('MDT_MATH_SINGLE_DOLLAR', false),
    docstrings: !/^(off|false|0|no)$/i.test(env('MDT_DOCSTRINGS') ?? 'on'),
    codeComments: bool('MDT_CODE_COMMENTS', true),
    frontMatter: bool('MDT_FRONT_MATTER', true),
  };
}

export interface LanguageConfig {
  code: string;
  name: string;
  translator?: string;
  formal: string;
  informal: string;
  style?: string;
  wrap?: 'words' | 'none';
  lengthRatio?: [number, number];
}

export interface LanguageCatalog {
  defaultTargets: string[];
  languages: Map<string, LanguageConfig>;
}

export function loadLanguages(file: string): LanguageCatalog {
  const raw = JSON.parse(readFileSync(file, 'utf8')) as { defaultTargets: string[]; languages: Record<string, Omit<LanguageConfig, 'code'>> };
  const languages = new Map<string, LanguageConfig>();
  for (const [code, l] of Object.entries(raw.languages)) languages.set(code.toLowerCase(), { code, ...l });
  for (const d of raw.defaultTargets) if (!languages.has(d.toLowerCase())) throw new Error(`default target "${d}" missing in ${file}`);
  return { defaultTargets: raw.defaultTargets, languages };
}

export interface Glossary {
  doNotTranslate: string[];
  terms: Record<string, Record<string, string>>;
}

export function loadGlossary(file: string): Glossary {
  const raw = JSON.parse(readFileSync(file, 'utf8')) as Partial<Glossary>;
  return { doNotTranslate: raw.doNotTranslate ?? [], terms: raw.terms ?? {} };
}
