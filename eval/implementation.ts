/**
 * The implementation under test. EVAL_IMPL=typescript evaluates the TypeScript pipeline in src/ (eval/typescript.ts);
 * any other value, and no value, evaluates the Rust binary (eval/rust.ts). Both return the same shapes: what the
 * binary writes with -dumpExtraction and -reportFile.
 */

export interface Switches {
  review: boolean;
  structuralContext: boolean;
  nmtFallback: boolean;
  preserveAnchors: boolean;
  codeComments: boolean;
  docstrings: boolean;
  frontMatter: boolean;
  mdx: boolean;
  mathSingleDollar: boolean;
}

export interface SegmentInfo {
  id: string;
  kind: string;
  textContext: string;
  note: string;
  structure: string | null;
  masked: string;
  original: string;
  passive: boolean;
}

export interface ExtractionDump {
  segments: SegmentInfo[];
  notes: string[];
  frontmatterFormality: 'formal' | 'informal' | null;
}

export interface Outcome {
  id: string;
  kind: string;
  via: 'cache' | 'gpt' | 'nmt' | 'source' | 'pseudo' | 'tm';
  retries: number;
  errors?: string[];
  review?: { category: string; severity: string; explanation: string; before?: string };
  untranslated?: boolean;
}

export interface LanguageReport {
  language: string;
  segments: number;
  via: Record<string, number>;
  retried: number;
  reviewEdits: number;
  revertedForStructure: string[];
  anchorsAdded: string[];
  keptSource: { id: string; errors: string[] }[];
  notes: string[];
  error?: string;
  tm: [string, string][];
  outcomes: Outcome[];
  segmentsDetail?: { id: string; kind: string; note: string; masked: string; original: string; rendered: string }[];
}

export interface Usage {
  promptTokens: number;
  completionTokens: number;
  calls: number;
}

export interface Report {
  file: string;
  engine: string;
  seconds: number;
  sourceLanguage: string;
  formality: 'formal' | 'informal';
  analysis: unknown;
  languages: LanguageReport[];
  usage: { byDeployment: Record<string, Usage>; byPurpose: Record<string, Usage> };
  /** Translated document. */
  text: string;
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

export interface TranslateOptions {
  formality: 'formal' | 'informal';
  analysis?: unknown;
  sourceLanguage?: string;
  cacheDir?: string;
  engine?: 'gpt' | 'nmt' | 'pseudo';
  translateDeployment?: string;
  reviewDeployment?: string;
  reportSegments?: boolean;
  /** Assemble from this translation memory instead of translating. */
  memory?: { tm: [string, string][]; outcomes: Outcome[]; review?: boolean };
}

export interface Implementation {
  /** Names the implementation and its prompt version; saved outputs are reused only while it stays the same. */
  version(): Promise<string>;
  languages(): Promise<{ defaultTargets: string[]; languages: Map<string, LanguageConfig> }>;
  extraction(file: string, s: Partial<Switches>): Promise<ExtractionDump>;
  /** The document analysis, or undefined when it failed. */
  analyze(file: string): Promise<unknown>;
  /** Translates (or, with `memory`, assembles) one document into one language. */
  translate(file: string, lang: string, s: Partial<Switches>, o: TranslateOptions): Promise<Report>;
  tokens(rep: Report, purpose: 'translations' | 'review' | 'analysis'): number;
  /** Model tokens of all runs so far, per deployment. */
  usage(): Record<string, Usage>;
}

const requested = (process.env.EVAL_IMPL ?? '').trim();
export const IMPLEMENTATION: 'rust' | 'typescript' = requested.toLowerCase() === 'typescript' ? 'typescript' : 'rust';
if (requested && IMPLEMENTATION === 'rust' && requested.toLowerCase() !== 'rust') {
  console.warn(`EVAL_IMPL=${requested} is neither "rust" nor "typescript"; evaluating the Rust binary`);
}

const impl: Implementation = IMPLEMENTATION === 'typescript' ? await import('./typescript.js') : await import('./rust.js');
export const { version, languages, extraction, analyze, translate, tokens, usage } = impl;
