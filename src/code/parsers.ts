import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);

export interface CommentRange {
  start: number;
  end: number;
}

// ---------------------------------------------------------------- tree-sitter

const TS_GRAMMARS: Record<string, string> = {
  javascript: 'javascript', js: 'javascript', jsx: 'javascript', mjs: 'javascript', cjs: 'javascript', node: 'javascript', nodejs: 'javascript',
  typescript: 'typescript', ts: 'typescript', mts: 'typescript', cts: 'typescript', tsx: 'tsx',
  python: 'python', py: 'python', python3: 'python', py3: 'python', gyp: 'python',
  bash: 'bash', sh: 'bash', shell: 'bash', zsh: 'bash', ksh: 'bash', shellscript: 'bash',
  powershell: 'powershell', ps1: 'powershell', pwsh: 'powershell', ps: 'powershell', posh: 'powershell', psm1: 'powershell',
  csharp: 'c-sharp', cs: 'c-sharp', 'c#': 'c-sharp',
  cpp: 'cpp', 'c++': 'cpp', cxx: 'cpp', cc: 'cpp', hpp: 'cpp', c: 'cpp', h: 'cpp', arduino: 'cpp', cuda: 'cpp',
  css: 'css',
  go: 'go', golang: 'go',
  java: 'java',
  php: 'php',
  ruby: 'ruby', rb: 'ruby',
  rust: 'rust', rs: 'rust',
  ini: 'ini', cfg: 'ini', dosini: 'ini', properties: 'ini', editorconfig: 'ini', gitconfig: 'ini',
};

interface TsNode {
  type: string;
  startIndex: number;
  endIndex: number;
  parent: TsNode | null;
  descendantsOfType(types: string | string[]): (TsNode | null)[];
}
interface TsParser {
  setLanguage(l: unknown): void;
  parse(text: string): { rootNode: TsNode; delete(): void } | null;
}

let tsModule: { Parser: { new (): TsParser; init(o: unknown): Promise<void> }; Language: { load(b: Uint8Array): Promise<unknown> } } | null = null;
const tsLanguages = new Map<string, unknown>();
const COMMENT_TYPES = ['comment', 'line_comment', 'block_comment', 'doc_comment'];

export async function initCodeParsers(): Promise<void> {
  if (tsModule) return;
  const mod = require('@vscode/tree-sitter-wasm');
  const wasmDir = join(dirname(require.resolve('@vscode/tree-sitter-wasm')));
  await mod.Parser.init({ locateFile: (file: string) => join(wasmDir, file) });
  for (const grammar of new Set(Object.values(TS_GRAMMARS))) {
    tsLanguages.set(grammar, await mod.Language.load(readFileSync(join(wasmDir, `tree-sitter-${grammar}.wasm`))));
  }
  tsModule = mod;
}

function treeSitterComments(grammar: string, code: string): CommentRange[] {
  if (!tsModule) throw new Error('initCodeParsers() must be awaited first');
  const parser = new tsModule.Parser();
  parser.setLanguage(tsLanguages.get(grammar));
  const tree = parser.parse(code);
  if (!tree) return [];
  try {
    return tree.rootNode
      .descendantsOfType(COMMENT_TYPES)
      .filter((n): n is TsNode => !!n && !(n.parent && COMMENT_TYPES.includes(n.parent.type)))
      .map((n) => ({ start: n.startIndex, end: n.endIndex }))
      .sort((a, b) => a.start - b.start);
  } finally {
    tree.delete();
  }
}

// ---------------------------------------------------------------- lexer fallback

interface StrSpec {
  open: string;
  close: string;
  esc?: string;
  doubled?: boolean;
}
interface LexSpec {
  line: string[];
  block: [string, string][];
  strings: StrSpec[];
  lineStartOnly?: boolean;
  /** Line comment marker must follow whitespace or line start (YAML, shell-like). */
  lineNeedsSpace?: boolean;
}

const DQ: StrSpec = { open: '"', close: '"', esc: '\\' };
const SQ: StrSpec = { open: "'", close: "'", esc: '\\' };
const C_LIKE: LexSpec = { line: ['//'], block: [['/*', '*/']], strings: [{ open: '"""', close: '"""' }, DQ, SQ, { open: '`', close: '`', esc: '\\' }] };

const LEX: Record<string, LexSpec> = {
  bicep: { line: ['//'], block: [['/*', '*/']], strings: [{ open: "'''", close: "'''" }, SQ] },
  terraform: { line: ['#', '//'], block: [['/*', '*/']], strings: [DQ] },
  sql: { line: ['--'], block: [['/*', '*/']], strings: [{ open: "'", close: "'", doubled: true }, { open: '"', close: '"', doubled: true }, { open: '[', close: ']' }] },
  kusto: { line: ['//'], block: [], strings: [DQ, SQ, { open: '```', close: '```' }] },
  yaml: { line: ['#'], block: [], strings: [DQ, { open: "'", close: "'", doubled: true }], lineNeedsSpace: true },
  toml: { line: ['#'], block: [], strings: [{ open: '"""', close: '"""', esc: '\\' }, { open: "'''", close: "'''" }, DQ, { open: "'", close: "'" }] },
  dockerfile: { line: ['#'], block: [], strings: [], lineStartOnly: true },
  makefile: { line: ['#'], block: [], strings: [], lineNeedsSpace: true },
  hash: { line: ['#'], block: [], strings: [DQ, SQ], lineNeedsSpace: true },
  xml: { line: [], block: [['<!--', '-->']], strings: [] },
  lua: { line: ['--'], block: [['--[[', ']]']], strings: [DQ, SQ, { open: '[[', close: ']]' }] },
  haskell: { line: ['--'], block: [['{-', '-}']], strings: [DQ] },
  vb: { line: ["'"], block: [], strings: [{ open: '"', close: '"', doubled: true }] },
  batch: { line: ['REM ', 'rem ', '::'], block: [], strings: [], lineStartOnly: true },
  clike: C_LIKE,
  scss: { line: ['//'], block: [['/*', '*/']], strings: [DQ, SQ] },
  graphql: { line: ['#'], block: [], strings: [{ open: '"""', close: '"""' }, DQ] },
  erlang: { line: ['%'], block: [], strings: [DQ] },
  lisp: { line: [';'], block: [], strings: [DQ] },
};

const LEX_ALIASES: Record<string, string> = {
  bicep: 'bicep', bicepparam: 'bicep',
  hcl: 'terraform', terraform: 'terraform', tf: 'terraform', tfvars: 'terraform',
  sql: 'sql', tsql: 'sql', 't-sql': 'sql', mysql: 'sql', postgresql: 'sql', postgres: 'sql', psql: 'sql', plsql: 'sql', sqlite: 'sql', mssql: 'sql', pgsql: 'sql',
  kql: 'kusto', kusto: 'kusto', csl: 'kusto',
  yaml: 'yaml', yml: 'yaml',
  toml: 'toml',
  dockerfile: 'dockerfile', docker: 'dockerfile', containerfile: 'dockerfile',
  makefile: 'makefile', make: 'makefile', mk: 'makefile',
  r: 'hash', perl: 'hash', pl: 'hash', elixir: 'hash', ex: 'hash', exs: 'hash', nginx: 'hash', apache: 'hash', conf: 'hash', cmake: 'hash', julia: 'hash', jl: 'hash', nim: 'hash', tcl: 'hash', gitignore: 'hash', dotenv: 'hash', env: 'hash', coffee: 'hash', crystal: 'hash', awk: 'hash', fish: 'hash', starlark: 'hash', bazel: 'hash', gdscript: 'hash',
  xml: 'xml', html: 'xml', xhtml: 'xml', svg: 'xml', xaml: 'xml', csproj: 'xml', msbuild: 'xml', plist: 'xml', vue: 'xml', razor: 'xml', cshtml: 'xml',
  lua: 'lua',
  haskell: 'haskell', hs: 'haskell', elm: 'haskell',
  vb: 'vb', vbnet: 'vb', 'vb.net': 'vb', vba: 'vb', vbscript: 'vb', vbs: 'vb',
  bat: 'batch', batch: 'batch', cmd: 'batch',
  kotlin: 'clike', kt: 'clike', kts: 'clike', swift: 'clike', scala: 'clike', dart: 'clike', groovy: 'clike', gradle: 'clike', jsonc: 'clike', json5: 'clike', proto: 'clike', protobuf: 'clike',
  solidity: 'clike', sol: 'clike', zig: 'clike', objectivec: 'clike', objc: 'clike', 'objective-c': 'clike', fsharp: 'clike', fs: 'clike', glsl: 'clike', hlsl: 'clike', wgsl: 'clike', d: 'clike', v: 'clike', verilog: 'clike', apex: 'clike',
  scss: 'scss', less: 'scss', sass: 'scss', stylus: 'scss',
  graphql: 'graphql', gql: 'graphql',
  erlang: 'erlang', erl: 'erlang', matlab: 'erlang', octave: 'erlang', latex: 'erlang', tex: 'erlang',
  lisp: 'lisp', clojure: 'lisp', clj: 'lisp', scheme: 'lisp', racket: 'lisp', elisp: 'lisp', asm: 'lisp', nasm: 'lisp', ini2: 'lisp',
};

function skipString(code: string, i: number, s: StrSpec): number {
  while (i < code.length) {
    if (s.esc && code[i] === s.esc) {
      i += 2;
      continue;
    }
    if (code.startsWith(s.close, i)) {
      if (s.doubled && code.startsWith(s.close, i + s.close.length)) {
        i += s.close.length * 2;
        continue;
      }
      return i + s.close.length;
    }
    // Single-line strings end at line breaks; this keeps a stray quote from swallowing the file.
    if (code[i] === '\n' && s.open.length === 1 && s.open !== '`' && s.open !== '[') return i;
    i++;
  }
  return i;
}

function lexComments(code: string, spec: LexSpec): CommentRange[] {
  const out: CommentRange[] = [];
  let atLineStart = true;
  let i = 0;
  while (i < code.length) {
    const c = code[i];
    if (c === '\n') {
      atLineStart = true;
      i++;
      continue;
    }
    const block = spec.block.find(([o]) => code.startsWith(o, i));
    if (block && (!spec.lineStartOnly || atLineStart)) {
      const close = code.indexOf(block[1], i + block[0].length);
      const end = close === -1 ? code.length : close + block[1].length;
      out.push({ start: i, end });
      i = end;
      atLineStart = false;
      continue;
    }
    const line = spec.line.find((o) => code.startsWith(o, i));
    const spaceOk = !spec.lineNeedsSpace || i === 0 || /\s/.test(code[i - 1]);
    if (line && (!spec.lineStartOnly || atLineStart) && spaceOk) {
      let end = code.indexOf('\n', i);
      if (end === -1) end = code.length;
      if (code[end - 1] === '\r') end--;
      out.push({ start: i, end });
      i = end;
      continue;
    }
    const str = spec.strings.find((s) => code.startsWith(s.open, i));
    if (str) {
      i = skipString(code, i + str.open.length, str);
      atLineStart = false;
      continue;
    }
    if (!/\s/.test(c)) atLineStart = false;
    i++;
  }
  return out;
}

/** Comment ranges in `code`, or null when the language is not supported (block is then left untouched). */
export function findComments(lang: string, code: string): CommentRange[] | null {
  const key = lang.trim().toLowerCase().replace(/^\{?\.?/, '').split(/[\s{,]/)[0];
  const grammar = TS_GRAMMARS[key];
  if (grammar) return treeSitterComments(grammar, code);
  const lex = LEX_ALIASES[key];
  if (lex) return lexComments(code, LEX[lex]);
  return null;
}

export function isSupportedCodeLanguage(lang: string): boolean {
  const key = lang.trim().toLowerCase().replace(/^\{?\.?/, '').split(/[\s{,]/)[0];
  return key in TS_GRAMMARS || key in LEX_ALIASES;
}

/** Code with comments removed and whitespace collapsed; used to prove code bytes were not altered. */
export function codeFingerprint(lang: string, code: string): string | null {
  const comments = findComments(lang, code);
  if (!comments) return null;
  let out = '';
  let pos = 0;
  for (const c of comments) {
    out += code.slice(pos, c.start) + ' ';
    pos = c.end;
  }
  return (out + code.slice(pos)).replace(/\s+/g, ' ').trim() + `\u0000${comments.length > 0 ? 'c' : ''}`;
}
