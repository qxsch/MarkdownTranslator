// Ad-hoc: translate files from the command line. Usage: tsx scripts/translate.ts <lang[,lang]> <file.md>... [--no-review] [--nmt]
import { existsSync, readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';

if (existsSync('.env')) process.loadEnvFile('.env');
const { MarkdownTranslator } = await import('../src/pipeline.js');

const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith('--')));
const [langs, ...paths] = args.filter((a) => !a.startsWith('--'));
const t = await MarkdownTranslator.create();
const files = Object.fromEntries(paths.map((p) => [basename(p), readFileSync(p, 'utf8')]));
const started = Date.now();
const res = await t.translateFiles(files, langs.split(','), {
  review: !flags.has('--no-review'),
  engine: flags.has('--nmt') ? 'nmt' : 'gpt',
  docstrings: flags.has('--docstrings') || undefined,
  structuralContext: flags.has('--no-structure') ? false : undefined,
});
mkdirSync('out', { recursive: true });
for (const [lang, byFile] of Object.entries(res.translations)) {
  for (const [file, text] of Object.entries(byFile)) writeFileSync(join('out', file.replace(/(\.[^.]+)$/, `.${lang}$1`)), text);
}
console.log(JSON.stringify({ seconds: (Date.now() - started) / 1000, reports: res.reports, analyses: res.analyses, usage: res.usage }, null, 2));
