/**
 * Detection of spans that must never be translated inside running text:
 * URLs, file names, paths, identifiers, variables, CLI flags, versions, etc.
 */
import { findDollarMath } from '../markdown/mathSpans.js';

const FILE_EXTENSIONS = [
  'md', 'mdx', 'markdown', 'txt', 'rst', 'adoc', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'csv', 'tsv', 'rtf', 'odt',
  'png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'bmp', 'ico', 'tif', 'tiff', 'avif', 'heic', 'psd', 'excalidraw', 'drawio', 'vsdx',
  'mp3', 'mp4', 'wav', 'mov', 'avi', 'mkv', 'webm', 'ogg', 'flac',
  'zip', 'tar', 'gz', 'tgz', 'bz2', 'xz', '7z', 'rar', 'nupkg', 'whl', 'jar', 'war', 'ear', 'deb', 'rpm', 'msi', 'exe', 'dll', 'so', 'dylib', 'bin', 'iso', 'vhd', 'vhdx', 'img', 'dmg', 'apk', 'appx', 'msix',
  'js', 'mjs', 'cjs', 'jsx', 'ts', 'mts', 'cts', 'tsx', 'vue', 'svelte', 'astro', 'json', 'jsonc', 'json5', 'jsonl', 'ndjson', 'yaml', 'yml', 'toml', 'ini', 'cfg', 'conf', 'config', 'env', 'properties', 'xml', 'xsd', 'xsl', 'xslt', 'plist',
  'html', 'htm', 'xhtml', 'css', 'scss', 'sass', 'less', 'styl',
  'py', 'pyi', 'pyc', 'ipynb', 'r', 'rmd', 'jl', 'rb', 'erb', 'php', 'pl', 'pm', 'lua', 'go', 'rs', 'java', 'kt', 'kts', 'scala', 'groovy', 'gradle', 'swift', 'm', 'mm', 'c', 'h', 'cc', 'cpp', 'cxx', 'hpp', 'hh', 'cs', 'csx', 'fs', 'fsx', 'vb', 'dart', 'ex', 'exs', 'erl', 'hs', 'clj', 'elm', 'zig', 'nim', 'sol',
  'sh', 'bash', 'zsh', 'fish', 'ps1', 'psm1', 'psd1', 'ps1xml', 'bat', 'cmd', 'vbs', 'awk', 'sed',
  'sql', 'db', 'sqlite', 'bak', 'mdf', 'ldf', 'parquet', 'avro', 'orc', 'onnx', 'pt', 'pth', 'pkl', 'h5', 'safetensors', 'gguf',
  'bicep', 'bicepparam', 'tf', 'tfvars', 'tfstate', 'hcl', 'nomad', 'arm', 'template', 'dockerfile', 'containerfile', 'helmignore', 'tpl',
  'csproj', 'vbproj', 'fsproj', 'sln', 'slnx', 'props', 'targets', 'nuspec', 'vcxproj', 'pbxproj', 'xcodeproj', 'resx', 'razor', 'cshtml', 'xaml', 'axaml',
  'lock', 'log', 'pem', 'crt', 'cer', 'key', 'pfx', 'p12', 'jks', 'pub', 'gpg', 'asc', 'sig', 'rules', 'service', 'socket', 'timer', 'desktop',
  'woff', 'woff2', 'ttf', 'otf', 'eot', 'map', 'wasm', 'proto', 'graphql', 'gql', 'prisma', 'http', 'rest', 'har', 'pbix', 'pbit', 'kql', 'csl',
];

const DOTFILES = [
  'env', 'gitignore', 'gitattributes', 'gitmodules', 'gitkeep', 'editorconfig', 'npmrc', 'nvmrc', 'yarnrc', 'dockerignore', 'prettierrc', 'prettierignore',
  'eslintrc', 'eslintignore', 'babelrc', 'browserslistrc', 'stylelintrc', 'vscode', 'github', 'git', 'bashrc', 'zshrc', 'profile', 'bash_profile', 'devcontainer',
  'azure', 'terraform', 'terraformrc', 'kube', 'ssh', 'aws', 'config', 'pylintrc', 'flake8', 'coveragerc', 'markdownlint', 'vsconfig', 'funcignore', 'venv',
];

const SPECIAL_FILES = [
  'Dockerfile', 'Containerfile', 'Makefile', 'Jenkinsfile', 'Vagrantfile', 'Procfile', 'Gemfile', 'Rakefile', 'Brewfile', 'Pipfile', 'Justfile', 'Taskfile',
  'CODEOWNERS', 'LICENSE', 'README', 'CHANGELOG', 'CONTRIBUTING', 'SECURITY', 'NOTICE', 'AUTHORS', 'OWNERS', 'MAINTAINERS', 'go.mod', 'go.sum', 'Cargo.toml',
];

const PS_VERBS = [
  'Add', 'Approve', 'Assert', 'Backup', 'Block', 'Build', 'Checkpoint', 'Clear', 'Close', 'Compare', 'Complete', 'Compress', 'Confirm', 'Connect', 'Convert',
  'ConvertFrom', 'ConvertTo', 'Copy', 'Debug', 'Deny', 'Deploy', 'Disable', 'Disconnect', 'Dismount', 'Edit', 'Enable', 'Enter', 'Exit', 'Expand', 'Export',
  'Find', 'ForEach', 'Format', 'Get', 'Grant', 'Group', 'Hide', 'Import', 'Initialize', 'Install', 'Invoke', 'Join', 'Limit', 'Lock', 'Measure', 'Merge',
  'Mount', 'Move', 'New', 'Open', 'Optimize', 'Out', 'Ping', 'Pop', 'Protect', 'Publish', 'Push', 'Read', 'Receive', 'Redo', 'Register', 'Remove', 'Rename',
  'Repair', 'Request', 'Reset', 'Resize', 'Resolve', 'Restart', 'Restore', 'Resume', 'Revoke', 'Save', 'Search', 'Select', 'Send', 'Set', 'Show', 'Skip',
  'Sort', 'Split', 'Start', 'Step', 'Stop', 'Submit', 'Suspend', 'Switch', 'Sync', 'Tee', 'Test', 'Trace', 'Unblock', 'Undo', 'Uninstall', 'Unlock',
  'Unprotect', 'Unpublish', 'Unregister', 'Update', 'Use', 'Wait', 'Watch', 'Where', 'Write',
];

const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

// Path/identifier "word" characters; markdown escapes like "\_" are allowed inside identifiers.
const W = String.raw`(?:[\w$@+~-]|\\[_*])`;
const BOUNDARY_END = String.raw`(?=$|[\s"'\`)\]}>,;:!?]|\.(?:$|\s))`;

interface Rule {
  name: string;
  re: RegExp;
  /** Group index that holds the protected span (defaults to whole match). */
  group?: number;
}

const RULES: Rule[] = [
  // GitHub / Microsoft Docs alerts and directives: > [!NOTE], > [!div class="..."]
  { name: 'alert', re: /\[![A-Za-z]+(?:\s[^\]\n]*)?\]/g },
  // Microsoft Docs includes and embeds: [!INCLUDE [title](path)], [!VIDEO url]
  { name: 'docsinclude', re: /\[!(?:INCLUDE|VIDEO|div|code)\b\s*/gi },
  { name: 'url', re: /\b(?:https?|ftp|ftps|sftp|file|ssh|git|s3|wss?|abfss?|wasbs?|vscode|mailto|tel|data|urn):(?:\/\/)?[^\s<>"'`]+[^\s<>"'`.,;:!?)\]}]/gi },
  { name: 'www', re: /\bwww\.[\w-]+(?:\.[\w-]+)+(?:\/[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}])?/gi },
  { name: 'email', re: /\b[\w.+-]+@[\w-]+(?:\.[\w-]+)+\b/g },
  { name: 'uncpath', re: /\\\\[\w.$-]+(?:\\[^\s\\<>"'`|*?]+)+\\?/g },
  { name: 'winpath', re: /\b[A-Za-z]:\\(?:[^\s\\<>"'`|*?]+\\?)*/g },
  { name: 'envpath', re: /(?:%[A-Za-z_][\w]*%|\$env:[A-Za-z_]\w*|\$\{?[A-Za-z_]\w*\}?)(?:[\\/][^\s<>"'`|*?]+)+/g },
  { name: 'abspath', re: new RegExp(String.raw`(?<![\w/\\.])(?:~|\.{1,2})?/${W}+(?:[./]${W}+)*/?${BOUNDARY_END}`, 'g') },
  // "and/or" style word pairs are prose; a relative path needs an extension or at least two separators.
  { name: 'relpath', re: new RegExp(String.raw`(?<![\w/\\])${W}+(?:\.${W}+)*(?:/${W}+(?:\.${W}+)*)*/${W}+\.[A-Za-z0-9]{1,10}${BOUNDARY_END}`, 'g') },
  { name: 'deeppath', re: new RegExp(String.raw`(?<![\w/\\])${W}+(?:\.${W}+)*(?:/${W}+(?:\.${W}+)*){2,}/?${BOUNDARY_END}`, 'g') },
  { name: 'dirpath', re: new RegExp(String.raw`(?<![\w/\\])${W}+(?:\.${W}+)*/${BOUNDARY_END}`, 'g') },
  { name: 'filename', re: new RegExp(String.raw`(?<![\w/\\.-])${W}+(?:\.${W}+)*\.(?:${FILE_EXTENSIONS.map(esc).join('|')})(?![\w-])`, 'gi') },
  { name: 'dotfile', re: new RegExp(String.raw`(?<![\w.])\.(?:${DOTFILES.map(esc).join('|')})(?:\.[\w-]+)*(?![\w-])`, 'g') },
  { name: 'specialfile', re: new RegExp(String.raw`\b(?:${SPECIAL_FILES.map(esc).join('|')})(?![\w-])`, 'g') },
  { name: 'template', re: /\{\{[^{}\n]{1,120}\}\}|\$\{[^{}\n]{1,120}\}|\{\{?[A-Za-z_][\w.-]*\}\}?|\{\d+(?::[^{}]*)?\}|<%[=-]?[^%]{1,120}%>|\[\[[^\]\n]{1,80}\]\]/g },
  { name: 'envvar', re: /\$env:[A-Za-z_][\w]*|%[A-Za-z_][\w]*%|\$[A-Za-z_][\w]*|\$\{[A-Za-z_]\w*\}/g },
  { name: 'printf', re: /(?<![\w%])%(?:\d+\$)?[-+0#]*\d*(?:\.\d+)?[sdifuxXoecg](?![\w])/g },
  { name: 'emoji', re: /(?<![\w:]):[a-z0-9_+-]{2,40}:(?![\w:])/g },
  { name: 'longflag', re: /(?<![\w-])--[A-Za-z0-9][\w-]*(?:=[^\s<>"'`]+)?/g },
  { name: 'shortflag', re: /(?<=^|[\s([])-[A-Za-z]{1,2}(?=$|[\s,.;:)\]])/g },
  { name: 'guid', re: /\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b/g },
  { name: 'version', re: /\bv\d+(?:\.\d+)+(?:[-+][\w.-]+)?\b|\b\d+\.\d+\.\d+(?:\.\d+)?(?:[-+][\w.-]+)?\b/g },
  { name: 'ip', re: /\b(?:\d{1,3}\.){3}\d{1,3}(?:\/\d{1,2})?(?::\d{1,5})?\b/g },
  { name: 'hostport', re: /\b(?:localhost|127\.0\.0\.1|0\.0\.0\.0)(?::\d{1,5})?\b/g },
  { name: 'hex', re: /(?<![\w#])#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b|\b0x[0-9a-fA-F]+\b/g },
  { name: 'issue', re: /(?<![\w&#])#\d+\b/g },
  { name: 'mention', re: /(?<![\w@.])@[A-Za-z0-9][\w-]*(?:\/[\w.-]+)?/g },
  { name: 'cmdlet', re: new RegExp(String.raw`\b(?:${PS_VERBS.join('|')})-[A-Z][A-Za-z0-9]+\b`, 'g') },
  { name: 'call', re: /\b[A-Za-z_$][\w$]*(?:(?:\.|::|->)[A-Za-z_$][\w$]*)*\(\)/g },
  { name: 'qualified', re: /\b[A-Za-z_][\w]*(?:(?:::|->)[A-Za-z_][\w]*)+\b|\b[A-Z][\w]*(?:\.[A-Z][\w]*){1,}\b|\b[a-z_][\w]*(?:\.[a-z_][\w]*)+\b(?=\(|\s*=)/g },
  { name: 'snake', re: /\b[A-Za-z][A-Za-z0-9]*(?:\\?_[A-Za-z0-9]+)+\b/g },
  { name: 'camel', re: /\b[a-z]+[0-9]*(?:[A-Z][a-z0-9]*)+\b/g },
  { name: 'pascal', re: /\b[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]*)+\b|\b[A-Z]{2,}[a-z]+[A-Za-z0-9]*\b/g },
];

export interface Span {
  start: number;
  end: number;
  rule: string;
}

export function findProtected(text: string, doNotTranslate: readonly string[] = []): Span[] {
  const spans: Span[] = [];
  const collect = (re: RegExp, rule: string) => {
    re.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text))) {
      if (m[0].length === 0) {
        re.lastIndex++;
        continue;
      }
      spans.push({ start: m.index, end: m.index + m[0].length, rule });
    }
  };
  for (const r of RULES) collect(r.re, r.name);
  // Formulas such as $\alpha = 0.7$ stay verbatim even when single-dollar math is not parsed.
  for (const m of findDollarMath(text)) spans.push({ ...m, rule: 'math' });
  for (const term of doNotTranslate) {
    if (!term.trim()) continue;
    const re = new RegExp(String.raw`(?<![\p{L}\p{N}_])${esc(term)}(?![\p{L}\p{N}_])`, 'gu');
    collect(re, 'dnt');
  }
  return mergeSpans(spans);
}

/** Keeps non-overlapping spans; the earliest start wins, then the longest. */
export function mergeSpans(spans: Span[]): Span[] {
  spans.sort((a, b) => a.start - b.start || b.end - a.end);
  const out: Span[] = [];
  for (const s of spans) {
    const last = out[out.length - 1];
    if (last && s.start < last.end) continue;
    out.push(s);
  }
  return out;
}
