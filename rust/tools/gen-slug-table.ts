import { regex } from 'github-slugger/regex.js';
import { writeFileSync } from 'node:fs';
const re = new RegExp(regex.source, 'g');
const ranges: [number, number][] = [];
let partial = 0;
for (let cp = 0; cp <= 0x10ffff; cp++) {
  if (cp >= 0xd800 && cp <= 0xdfff) continue;
  const s = String.fromCodePoint(cp);
  const r = s.replace(re, '');
  if (r === s) continue;
  if (r !== '') { partial++; continue; }
  const last = ranges[ranges.length - 1];
  if (last && last[1] === cp - 1) last[1] = cp;
  else if (last && last[1] === 0xd7ff && cp === 0xe000) last[1] = cp;
  else ranges.push([cp, cp]);
}
const body = ranges.map(([a, b]) => `    (0x${a.toString(16).toUpperCase()}, 0x${b.toString(16).toUpperCase()}),`).join('\n');
writeFileSync(process.argv[2], `//! Generated from github-slugger 2.0.0 \`regex.js\` (every code point the slug regex removes).\n//! Regenerate with \`npx tsx rust/tools/gen-slug-table.ts rust/src/markdown/slug_table.rs\`.\n\npub const REMOVED: &[(u32, u32)] = &[\n${body}\n];\n`);
console.log(`ranges: ${ranges.length}, partial: ${partial}`);
