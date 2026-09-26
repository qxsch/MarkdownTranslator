/**
 * Bridge from the evaluation framework to the Rust implementation (rust/): translations, extraction
 * fingerprints, document analyses and assembly all come from the `mdtranslate` binary. The TypeScript code in
 * src/ is only used for the independent checks (structure, code, links, English left behind) and for the
 * Foundry judges.
 *
 * Build the binary first:  cargo build --release --manifest-path rust/Cargo.toml
 * Or point to one:         MDT_RUST_BIN=/path/to/mdtranslate
 * Parallel processes:      EVAL_PARALLEL (default 8); Azure requests per process are MDT_MAX_CONCURRENCY / EVAL_PARALLEL.
 */
import { execFile } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
export const RUST_BIN = process.env.MDT_RUST_BIN ?? join(root, 'rust', 'target', 'release', process.platform === 'win32' ? 'mdtranslate.exe' : 'mdtranslate');

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

export interface RustSegment {
  id: string;
  kind: string;
  textContext: string;
  note: string;
  structure: string | null;
  masked: string;
  original: string;
  passive: boolean;
}

export interface RustExtraction {
  segments: RustSegment[];
  notes: string[];
  frontmatterFormality: 'formal' | 'informal' | null;
}

export interface RustOutcome {
  id: string;
  kind: string;
  via: 'cache' | 'gpt' | 'nmt' | 'source' | 'pseudo' | 'tm';
  retries: number;
  errors?: string[];
  review?: { category: string; severity: string; explanation: string; before?: string };
  untranslated?: boolean;
}

export interface RustLanguageReport {
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
  outcomes: RustOutcome[];
  segmentsDetail?: { id: string; kind: string; note: string; masked: string; original: string; rendered: string }[];
}

interface Usage {
  promptTokens: number;
  completionTokens: number;
  calls: number;
}

export interface RustReport {
  file: string;
  engine: string;
  seconds: number;
  sourceLanguage: string;
  formality: 'formal' | 'informal';
  analysis: unknown;
  languages: RustLanguageReport[];
  usage: { byDeployment: Record<string, Usage>; byPurpose: Record<string, Usage> };
  /** Translated document (read from the target file). */
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

class Pool {
  private active = 0;
  private queue: (() => void)[] = [];
  constructor(private readonly max: number) {}
  async run<T>(fn: () => Promise<T>): Promise<T> {
    if (this.active >= this.max) await new Promise<void>((r) => this.queue.push(r));
    this.active++;
    try {
      return await fn();
    } finally {
      this.active--;
      this.queue.shift()?.();
    }
  }
}

const PARALLEL = Math.max(1, Number(process.env.EVAL_PARALLEL ?? 8));
const pool = new Pool(PARALLEL);
const perProcess = Math.max(2, Math.ceil(Number(process.env.MDT_MAX_CONCURRENCY ?? 16) / PARALLEL));
const totals: Record<string, Usage> = {};

function checkBinary() {
  if (!existsSync(RUST_BIN)) throw new Error(`Rust binary not found at ${RUST_BIN}; build it with "cargo build --release --manifest-path rust/Cargo.toml" or set MDT_RUST_BIN`);
}

/** Runs the binary. Exit codes 2 (translation failed, source written unchanged) and 3 (some segments kept in the
 * source language) are results for the evaluation, not errors; the report says what happened. */
export function run(args: string[]): Promise<{ code: number; stdout: string; stderr: string }> {
  checkBinary();
  return pool.run(
    () =>
      new Promise((resolve, reject) => {
        execFile(RUST_BIN, args, { maxBuffer: 1 << 28, env: { ...process.env, MDT_MAX_CONCURRENCY: String(perProcess) }, windowsHide: true }, (err, stdout, stderr) => {
          const code = err ? ((err as { code?: number }).code ?? 1) : 0;
          if (typeof code !== 'number' || ![0, 2, 3].includes(code)) reject(new Error(`mdtranslate ${args.join(' ')} failed (exit ${String(code)}): ${String(stderr).trim() || (err as Error).message}`));
          else resolve({ code, stdout: String(stdout), stderr: String(stderr) });
        });
      }),
  );
}

const flag = (name: string, on: boolean) => `-${name}=${on}`;

export function switchArgs(s: Partial<Switches>): string[] {
  return (Object.entries(s) as [keyof Switches, boolean | undefined][]).filter(([, v]) => v !== undefined).map(([k, v]) => flag(k, v!));
}

export async function version(): Promise<string> {
  // The binary keeps stdout for -targetFile - ; the version goes to stderr.
  return (await run(['-version'])).stderr.trim();
}

export async function languages(): Promise<{ defaultTargets: string[]; languages: Map<string, LanguageConfig> }> {
  const raw = JSON.parse((await run(['-listLanguages', '-targetFile', '-'])).stdout) as { defaultTargets: string[]; languages: LanguageConfig[] };
  return { defaultTargets: raw.defaultTargets, languages: new Map(raw.languages.map((l) => [l.code.toLowerCase(), l])) };
}

export async function extraction(file: string, s: Partial<Switches>): Promise<RustExtraction> {
  return JSON.parse((await run(['-sourceFile', file, '-targetFile', '-', '-dumpExtraction', ...switchArgs(s)])).stdout) as RustExtraction;
}

function addUsage(u: RustReport['usage'] | undefined) {
  for (const [dep, x] of Object.entries(u?.byDeployment ?? {})) {
    const t = (totals[dep] ??= { promptTokens: 0, completionTokens: 0, calls: 0 });
    t.promptTokens += x.promptTokens;
    t.completionTokens += x.completionTokens;
    t.calls += x.calls;
  }
}

/** Model tokens accumulated over every binary run of this process, per deployment. */
export function usage(): Record<string, Usage> {
  return totals;
}

export function tokens(rep: RustReport, purpose: 'translations' | 'review' | 'analysis'): number {
  const u = rep.usage?.byPurpose?.[purpose];
  return u ? u.promptTokens + u.completionTokens : 0;
}

export async function analyze(file: string): Promise<unknown> {
  const res = await run(['-sourceFile', file, '-targetFile', '-', '-analyzeOnly']);
  if (res.code !== 0) {
    // Exit 2: the analysis failed and stdout is empty. Like the TypeScript pipeline, go on without an analysis.
    console.warn(`document analysis of ${file} failed, continuing without it: ${res.stderr.trim()}`);
    return undefined;
  }
  const out = JSON.parse(res.stdout) as { analysis: unknown; usage: RustReport['usage'] };
  addUsage(out.usage);
  return out.analysis ?? undefined;
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
  memory?: { tm: [string, string][]; outcomes: RustOutcome[]; review?: boolean };
}

/** Translates (or, with `memory`, assembles) one document into one language. */
export async function translate(file: string, lang: string, s: Partial<Switches>, o: TranslateOptions): Promise<RustReport> {
  const dir = mkdtempSync(join(tmpdir(), 'mdt-eval-'));
  try {
    const target = join(dir, 'out.md');
    const report = join(dir, 'report.json');
    const args = ['-sourceFile', file, '-targetFile', target, '-lang', lang, '-reportFile', report, '-quiet', '-formality', o.formality, '-sourceLanguage', o.sourceLanguage ?? 'en', ...switchArgs(s)];
    if (o.analysis !== undefined) {
      writeFileSync(join(dir, 'analysis.json'), JSON.stringify({ analysis: o.analysis ?? null }));
      args.push('-analysisFile', join(dir, 'analysis.json'));
    }
    if (o.cacheDir) args.push('-cacheDir', o.cacheDir);
    if (o.engine) args.push('-engine', o.engine);
    if (o.translateDeployment) args.push('-translateDeployment', o.translateDeployment);
    if (o.reviewDeployment) args.push('-reviewDeployment', o.reviewDeployment);
    if (o.reportSegments) args.push('-reportSegments');
    if (o.memory) {
      writeFileSync(join(dir, 'tm.json'), JSON.stringify({ tm: o.memory.tm, outcomes: o.memory.outcomes }));
      args.push('-tmFile', join(dir, 'tm.json'));
      if (o.memory.review) args.push('-reviewTm');
    }
    await run(args);
    const rep = JSON.parse(readFileSync(report, 'utf8')) as RustReport;
    rep.text = readFileSync(target, 'utf8');
    addUsage(rep.usage);
    return rep;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}
