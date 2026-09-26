// Parser parity probe: dumps mdast node types with byte offsets, one `<out>/<name>.ts.txt` per input file.
//   npx tsx rust/tools/mdast-dump.ts <outDir> <file>... [--math]
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { parseMarkdown } from '../../src/markdown/parse.ts';

const [outDir, ...args] = process.argv.slice(2);
const math = args.includes('--math');
const files = args
  .filter((a) => !a.startsWith('--'))
  .flatMap((p) => (statSync(p).isDirectory() ? readdirSync(p).filter((f) => /\.mdx?$/i.test(f)).map((f) => join(p, f)) : [p]));
for (const file of files) {
  let src = readFileSync(file, 'utf8');
  if (src.startsWith('\uFEFF')) src = src.slice(1);
  const out: string[] = [];
  try {
    const tree = parseMarkdown(src, { mdx: /\.mdx$/i.test(file), mathSingleDollar: math });
    const byteAt = (o: number) => Buffer.byteLength(src.slice(0, o));
    const visit = (n: any, depth: number) => {
      const s = n.position?.start?.offset;
      const e = n.position?.end?.offset;
      let extra = '';
      if (n.type === 'text' || n.type === 'inlineCode' || n.type === 'html') extra = ' ' + JSON.stringify(n.value);
      if (n.type === 'link' || n.type === 'image' || n.type === 'definition') extra = ` url=${JSON.stringify(n.url)} title=${JSON.stringify(n.title ?? null)}`;
      if (n.type === 'code') extra = ` lang=${JSON.stringify(n.lang ?? null)} meta=${JSON.stringify(n.meta ?? null)}`;
      if (n.type === 'linkReference' || n.type === 'imageReference') extra = ` id=${n.identifier} ref=${n.referenceType}`;
      if (n.type === 'containerDirective' || n.type === 'leafDirective') extra = ` name=${n.name} attrs=${JSON.stringify(n.attributes)}`;
      if (n.type === 'paragraph' && n.data?.directiveLabel) extra = ' label';
      out.push(`${'  '.repeat(depth)}${n.type} ${s === undefined ? '?' : byteAt(s)}-${e === undefined ? '?' : byteAt(e)}${extra}`);
      for (const c of n.children ?? []) visit(c, depth + 1);
    };
    visit(tree, 0);
  } catch (e) {
    out.push(`error ${(e as Error).message}`);
  }
  writeFileSync(join(outDir, `${basename(file)}.ts.txt`), out.join('\n') + '\n');
}
