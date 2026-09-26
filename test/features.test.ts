import { readFileSync } from 'node:fs';
import { parse as parseYaml } from 'yaml';
import { describe, expect, it } from 'vitest';
import { initCodeParsers } from '../src/code/parsers.js';
import { isCode } from '../src/code/comments.js';
import { extract } from '../src/markdown/extract.js';
import { applyTranslations, compareSkeletons, skeleton } from '../src/markdown/document.js';
import { assembleDocument } from '../src/assemble.js';
import { checkTags } from '../src/mask/masking.js';
import { segmentPayload } from '../src/translate/prompts.js';
import { validateSegment } from '../src/translate/validate.js';
import type { ExtractOptions } from '../src/markdown/context.js';
import { LANG, pseudoMap, pseudoWords } from './helpers.js';

await initCodeParsers();

const fixture = (name: string) => readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8').replace(/\r\n/g, '\n');

function roundTrip(name: string, opts: ExtractOptions = {}) {
  const src = fixture(name);
  const ex = extract(src, [], opts);
  const identity = assembleDocument(ex, new Map(), { wrap: true, preserveAnchors: true });
  const tm = pseudoMap(ex);
  const failures = ex.segments
    .filter((s) => !s.passive)
    .map((s) => ({ s, e: validateSegment(s, tm.get(s.id)!, ex, LANG) }))
    .filter((x) => x.e.length)
    .map((x) => `${x.s.id} ${x.s.masked} -> ${x.e.join('; ')}`);
  const out = assembleDocument(ex, tm, { wrap: true, preserveAnchors: true });
  return { src, ex, identity, out, failures };
}

function commonChecks(r: ReturnType<typeof roundTrip>) {
  expect(r.identity.text).toBe(r.src);
  expect(r.failures).toEqual([]);
  expect(r.out.reverted).toEqual([]);
  expect(compareSkeletons(skeleton(r.src, r.ex.parseOptions), skeleton(r.out.text, r.ex.parseOptions))).toBe(-1);
}

describe('1. structural context', () => {
  const ex = extract(fixture('structure.md'), []);
  const seg = (text: string) => ex.segments.find((s) => s.masked === text || (text.length > 10 && s.masked.includes(text)))!;

  it('records the section path', () => {
    expect(seg('Before you deploy').structure).toContain('section "Settings > Configure the app"');
  });

  it('records the sentence that introduces a list', () => {
    expect(seg('Which region').structure).toContain('list introduced by "Before you deploy, consider the following questions:"');
  });

  it('records table column and row for cells', () => {
    expect(seg('Run').structure).toContain('table column "Default", row "mode"');
    expect(seg('Start behavior').structure).toContain('table column "Description"');
    expect(seg('Setting').structure).toContain('table header; columns: "Setting", "Default", "Description"');
  });

  it('is sent to the model only when enabled', () => {
    expect(segmentPayload(seg('Run'), true)).toHaveProperty('structure');
    expect(segmentPayload(seg('Run'), false)).not.toHaveProperty('structure');
  });
});

describe('2. YAML front matter via YAML AST', () => {
  const r = roundTrip('frontmatter.md');

  it('keeps bytes and structure', () => commonChecks(r));

  it('detects formality from front matter', () => {
    expect(r.ex.frontmatterFormality).toBe('informal');
  });

  it('translates nested keys, quoted, folded and literal scalars and keyword lists', () => {
    const fm = parseYaml(r.out.text.split('---')[1]);
    expect(fm.title).toBe(pseudoWords('Getting started'));
    expect(fm.description).toBe(`${pseudoWords('Learn how to deploy the service and configure it for production.')}\n${pseudoWords('A second paragraph of the description.')}\n`);
    expect(fm.summary).toBe(`${pseudoWords('First line of the summary.')}\n${pseudoWords('Second line of the summary.')}\n`);
    expect(fm.seo.title).toBe(pseudoWords('SEO: a title with "quotes"'));
    expect(fm.seo.og_description).toBe(pseudoWords("It's a great tool"));
    expect(fm.keywords).toEqual([pseudoWords('translation'), pseudoWords('markdown tools')]);
    expect(fm.nav[0].title).toBe(pseudoWords('Home page'));
    expect(fm.nav[0].url).toBe('/');
    expect(fm.tags).toEqual(['azure', 'markdown']);
    expect(fm.formality).toBe('informal');
  });

  it('keeps block scalar indicators and indentation', () => {
    expect(r.out.text).toContain('description: >\n  ÜLEARN');
    expect(r.out.text).toContain(`summary: |\n  ${pseudoWords('First line of the summary.')}\n  ${pseudoWords('Second line of the summary.')}\n`);
    expect(r.out.text).toContain(`  og_description: '${pseudoWords("It's a great tool").replace(/'/g, "''")}'`);
  });
});

describe('3. Markdown dialects', () => {
  const r = roundTrip('dialects.md');

  it('keeps bytes and structure', () => commonChecks(r));

  it.each([
    ['math stays untouched, prices are prose', `${pseudoWords('Costs')} $5 ${pseudoWords('and')} $10 ${pseudoWords('per unit')}, ${pseudoWords('and the formula')} $$e = mc^2$$ ${pseudoWords('stays')}.`],
    ['block math', '$$\n\\sum_{i=1}^n i\n$$'],
    ['container directive title', `:::note[${pseudoWords('Custom title')}]\n${pseudoWords('Some')} **${pseudoWords('important')}** ${pseudoWords('text')}.\n:::`],
    ['Docusaurus admonition title and line structure', `:::tip ${pseudoWords('Pro tip')}\n${pseudoWords('Use the cache')}.\n:::`],
    ['leaf directive label', `::youtube[${pseudoWords('Intro video')}]{#abc123}`],
    ['Docs image alt text only', `:::image type="content" source="media/arch.png" alt-text="${pseudoWords('Architecture diagram')}":::`],
    ['Docs zone fences', ':::zone pivot="windows"\n\n'],
    ['Docs zone end', ':::zone-end'],
    ['GitHub alert', '> [!NOTE]\n> '],
    ['Docs include', `[!INCLUDE [${pseudoWords('shared prerequisites')}](../includes/prereqs.md)]`],
    ['Hugo shortcode title only', `{{< figure src="images/overview.png" title="${pseudoWords('Architecture overview')}" >}}`],
    ['Hugo paired shortcodes', `{{% notice warning %}}\n${pseudoWords('Do not delete the resource group')}.\n{{% /notice %}}`],
  ])('%s', (_name, needle) => {
    expect(r.out.text).toContain(needle);
  });

  it('rejects translations that move text out of shortcode attributes or add quotes', () => {
    const seg = r.ex.segments.find((s) => s.masked.includes('Architecture overview'))!;
    const pair = [...seg.pairs.keys()][0];
    const xs = [...seg.placeholders.keys()];
    expect(checkTags(seg, `<x${xs[0]}/>Hallo <g${pair}>Überblick</g${pair}><x${xs[1]}/>`).join()).toMatch(/attribute values/);
    expect(checkTags(seg, `<x${xs[0]}/><g${pair}>Der "Überblick"</g${pair}><x${xs[1]}/>`).join()).toMatch(/must not contain "/);
    expect(checkTags(seg, `<x${xs[0]}/><g${pair}>Überblick</g${pair}><x${xs[1]}/>`)).toEqual([]);
  });

  it('keeps fence line order in Docusaurus admonitions', () => {
    const seg = r.ex.segments.find((s) => s.masked.includes('Pro tip'))!;
    const bad = seg.masked.replace(/(<g\d+>Pro tip<\/g\d+>)(<x\d+\/>)(<g\d+>Use the cache\.<\/g\d+>)/, '$3$2$1');
    expect(bad).not.toBe(seg.masked);
    expect(checkTags(seg, bad).join()).toMatch(/original order/);
  });

  it('treats $...$ as math only when enabled', () => {
    const on = extract('Costs $5 and $10 per unit.\n', [], { parse: { mathSingleDollar: true } });
    expect(on.segments[0].masked).not.toContain(' and ');
    const off = extract('Costs $5 and $10 per unit.\n', []);
    expect(off.segments[0].masked).toContain(' and ');
  });

  it('parses MDX (JSX, expressions, ESM) and translates text and prose attributes', () => {
    const m = roundTrip('component.mdx', { parse: { mdx: true } });
    commonChecks(m);
    for (const kept of ["import Tabs from '@theme/Tabs';", '<Tabs groupId="os">', 'value="win"', '{props.version}', 'type="warning" />', "export const meta = { title: 'Not translated' };", '<Highlight color="#25c2a0">']) {
      expect(m.out.text).toContain(kept);
    }
    expect(m.out.text).toContain(`<Tab label="${pseudoWords('Windows')}" value="win">`);
    expect(m.out.text).toContain(`${pseudoWords('Install the')} **${pseudoWords('tool')}** ${pseudoWords('with winget')}.`);
    expect(m.out.text).toContain(`<Highlight color="#25c2a0">${pseudoWords('green text')}</Highlight>`);
    expect(m.out.text).toContain(`<Callout title="${pseudoWords('Heads up')}" type="warning" />`);
  });
});

describe('4. tree-sitter code decisions and docstrings', () => {
  it.each([
    ['python', 'x = compute(y)', true],
    ['python', 'Returns the value (or None);', false],
    ['typescript', 'const total = sum(items);', true],
    ['typescript', 'Create the client (see docs).', false],
    ['bash', 'rm -rf build', true],
    ['bash', 'Install the dependencies', false],
    ['csharp', 'Gets the value of the property.', false],
  ])('%s: %s -> code=%s', (lang, text, expected) => {
    expect(isCode(lang, text)).toBe(expected);
  });

  const r = roundTrip('structure.md');

  it('keeps bytes and structure', () => commonChecks(r));

  it('keeps commented-out code and translates prose comments', () => {
    for (const kept of ['# x = compute(y)\n', '// const total = sum(items);\n', '# rm -rf build\n']) expect(r.out.text).toContain(kept);
    for (const t of ['Returns the value (or None);', 'print the greeting', 'Create the client (see docs).', 'Install the dependencies']) {
      expect(r.out.text).toContain(pseudoWords(t));
    }
  });

  it('keeps docstrings by default and reports them', () => {
    expect(r.out.text).toContain('"""Load the configuration file.\n\n    Returns the parsed settings.\n    """');
    expect(r.ex.notes.join()).toMatch(/docstring/);
  });

  it('translates docstrings when enabled, keeping quotes and indentation', () => {
    const d = roundTrip('structure.md', { docstrings: true });
    commonChecks(d);
    expect(d.out.text).toContain(`"""${pseudoWords('Load the configuration file.')}\n\n    ${pseudoWords('Returns the parsed settings.')}\n    """`);
    const seg = d.ex.segments.find((s) => s.note === 'python docstring')!;
    expect(validateSegment(seg, 'Lädt die Datei \\n', d.ex, LANG).join()).toMatch(/must not contain/);
  });

  it('never lets a docstring translation change code bytes', () => {
    const d = extract(fixture('structure.md'), [], { docstrings: true });
    const seg = d.segments.find((s) => s.note === 'python docstring')!;
    const out = applyTranslations(d, new Map([[seg.id, 'Lädt die Datei.']]));
    expect(out).toContain('def load(path):\n    """Lädt die Datei.\n');
    expect(out).toContain('    return path');
  });
});
