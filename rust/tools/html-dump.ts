// Parser parity probe: dumps parse5 fragment trees with source locations.
//   npx tsx rust/tools/html-dump.ts <outDir> <file.html>...
import { readFileSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { parseFragment } from 'parse5';

const [outDir, ...files] = process.argv.slice(2);
for (const file of files) {
  const html = readFileSync(file, 'utf8');
  const byteAt = (o: number) => Buffer.byteLength(html.slice(0, o));
  const frag = parseFragment(html, { sourceCodeLocationInfo: true });
  const out: string[] = [];
  const visit = (n: any, depth: number) => {
    const l = n.sourceCodeLocation;
    const loc = l ? `${byteAt(l.startOffset)}-${byteAt(l.endOffset)}` : '?';
    let label: string;
    if (n.nodeName === '#text') label = `#text ${JSON.stringify(html.slice(l.startOffset, l.endOffset))}`;
    else if (n.nodeName === '#comment') label = '#comment';
    else {
      const st = l?.startTag ? ` st=${byteAt(l.startTag.startOffset)}-${byteAt(l.startTag.endOffset)}` : '';
      const et = l?.endTag ? ` et=${byteAt(l.endTag.startOffset)}-${byteAt(l.endTag.endOffset)}` : '';
      label = `${n.tagName}${st}${et}`;
    }
    out.push(`${'  '.repeat(depth)}${label} ${loc}`);
    const kids = n.tagName === 'template' ? n.content.childNodes : n.childNodes;
    for (const c of kids ?? []) visit(c, depth + 1);
  };
  for (const c of frag.childNodes) visit(c, 0);
  writeFileSync(join(outDir, `${basename(file)}.ts.txt`), out.join('\n') + '\n');
}
