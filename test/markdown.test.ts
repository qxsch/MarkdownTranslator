import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { initCodeParsers } from '../src/code/parsers.js';
import { extract } from '../src/markdown/extract.js';
import { applyTranslations, compareSkeletons, skeleton } from '../src/markdown/document.js';
import { assembleDocument } from '../src/assemble.js';
import { tokenize, xmlEscape, xmlUnescape, checkTags } from '../src/mask/masking.js';
import { findProtected } from '../src/mask/protect.js';
import { safeLineStart } from '../src/markdown/wrap.js';
import { validateSegment } from '../src/translate/validate.js';
import { htmlToMasked, maskedToHtml } from '../src/azure/nmt.js';
import type { Extraction } from '../src/markdown/types.js';
import type { LanguageConfig } from '../src/config.js';

const fixture = (name: string) => readFileSync(new URL(`./fixtures/${name}`, import.meta.url), 'utf8').replace(/\r\n/g, '\n');
const DNT = ['Azure', 'Node.js', 'Docker Desktop'];
const LANG: LanguageConfig = { code: 'de', name: 'German', formal: '', informal: '' };

/** Pseudo translation: every word gets a prefix and is upper-cased; tags stay where they are. */
const pseudoWords = (s: string) => s.replace(/\p{L}+/gu, (w) => `Ü${w.toUpperCase()}`);
function pseudo(masked: string): string {
  return tokenize(masked)
    .map((t) => (t.t === 'text' ? xmlEscape(pseudoWords(xmlUnescape(t.v))) : t.t === 'x' ? `<x${t.n}/>` : t.t === 'open' ? `<g${t.n}>` : `</g${t.n}>`))
    .join('');
}

function pseudoMap(ex: Extraction) {
  return new Map(ex.segments.filter((s) => !s.passive).map((s) => [s.id, pseudo(s.masked)]));
}

await initCodeParsers();

describe('byte identity', () => {
  for (const name of ['identity.md', 'tricky.md']) {
    for (const variant of ['lf', 'crlf', 'bom'] as const) {
      it(`${name} (${variant}) is unchanged without translations`, () => {
        let src = fixture(name);
        if (variant === 'crlf') src = src.replace(/\n/g, '\r\n');
        if (variant === 'bom') src = '\uFEFF' + src;
        const ex = extract(src, DNT);
        const out = assembleDocument(ex, new Map(), { wrap: true, preserveAnchors: true });
        expect(out.text).toBe(src);
      });
    }
  }

  for (const variant of ['lf', 'crlf'] as const) {
    it(`re-rendering every segment through its placeholders reproduces the source exactly (${variant})`, () => {
      let src = fixture('identity.md');
      if (variant === 'crlf') src = src.replace(/\n/g, '\r\n');
      const ex = extract(src, DNT);
      expect(ex.segments.length).toBeGreaterThan(30);
      // Re-wrapped blocks may break lines at other positions than the author did; everything else must match exactly.
      for (const r of ex.replacements) {
        const original = ex.source.slice(r.start, r.end);
        const rendered = r.render(new Map(), { force: true });
        if (ex.byId.get(r.segmentIds[0])?.wrap) expect(rendered.replace(/\s+/g, ' ')).toBe(original.replace(/\s+/g, ' '));
        else expect(rendered).toBe(original);
      }
    });
  }
});

describe('pseudo translation', () => {
  const src = fixture('identity.md');
  const ex = extract(src, DNT);
  const tm = pseudoMap(ex);

  it('passes per-segment validation for every segment', () => {
    const failures = ex.segments.filter((s) => !s.passive).map((s) => ({ s, e: validateSegment(s, tm.get(s.id)!, ex, LANG) })).filter((x) => x.e.length);
    expect(failures.map((f) => `${f.s.id} ${f.s.masked} -> ${f.e.join('; ')}`)).toEqual([]);
  });

  const out = assembleDocument(ex, tm, { wrap: true, preserveAnchors: true });

  it('keeps the Markdown structure without reverting anything', () => {
    expect(out.reverted).toEqual([]);
    expect(compareSkeletons(skeleton(src), skeleton(out.text))).toBe(-1);
  });

  it.each([
    'test.jpeg', '`docs/images/`', './scripts/deploy.ps1', '(images/architecture.png "', '<https://learn.microsoft.com>', 'www.example.com', '`$HOME`', '%APPDATA%', '${VAR_NAME}',
    'README.md', '--port', '-v', '`npm install`', '<kbd>Ctrl</kbd>+<kbd>C</kbd>', '`MDT_REVIEW`', '\\|', 'Node.js 22', 'Docker Desktop 4.30',
    'const client = new TranslatorClient(endpoint);', 'const url = "https://example.com // not a comment";', '    cfg = load("config.yaml")  # ',
    '#!/usr/bin/env bash', 'echo "# not a comment"', 'New-AzResourceGroupDeployment -ResourceGroupName rg -TemplateFile main.bicep # ', 'param location string = resourceGroup().location',
    '{ "name": "no comments in json" }', '<!-- This HTML comment stays untouched -->', '<img src="images/logo.svg" alt="', '" width="120">', '<div align="center">',
    '[^1]', '&copy;', '&nbsp;', '\\*', '[docs]: https://learn.microsoft.com/azure "', 'author: Jane Doe', 'tags: [azure, markdown]', '| `PORT` | 8080 |',
    '|:--------|:-------:|------------:|', '- [x] ', '- [ ] ', '1. ', '\\\n',
  ])('keeps %s byte-identical', (needle) => {
    expect(out.text).toContain(needle);
  });

  it.each([
    [`# <a id="getting-started-with-mdtranslator"></a>${pseudoWords('Getting started with')} \`mdtranslator\``],
    [`**${pseudoWords('deploy')}**`],
    [`[Azure](https://azure.microsoft.com "Azure ${pseudoWords('home page')}")`],
    [`![${pseudoWords('Architecture diagram of the solution')}](images/architecture.png "${pseudoWords('High level architecture')}")`],
    [`alt="${pseudoWords('Company logo')}"`],
    [`<p>${pseudoWords('This paragraph is inside an')} <em>${pseudoWords('HTML block')}</em>`],
    [`// ${pseudoWords('Create the client for the translation service')}`],
    [`// ${pseudoWords('inline comment')}`],
    [`# ${pseudoWords('Read the configuration file first')}`],
    [`# ${pseudoWords('Install the dependencies')}`],
    [`  ${pseudoWords('Deploys the infrastructure to the resource group')}.`],
    [`// ${pseudoWords('Storage account for the translation cache')}`],
    [`title: ${pseudoWords('Getting started with the translator')}`],
    [`description: "${pseudoWords('Learn how to deploy the service')}: ${pseudoWords('step by step')}"`],
    [`| ${pseudoWords('Enables the second review pass')} \\| ${pseudoWords('optional')} |`],
    [`[^1]: ${pseudoWords('The footnote text is translated as well')}.`],
  ])('translates %s', (needle) => {
    expect(out.text).toContain(needle);
  });

  it('adds anchors so links to the original headings keep working', () => {
    expect(out.anchors).toContain('getting-started-with-mdtranslator');
    expect(out.text).toContain('# <a id="getting-started-with-mdtranslator"></a>');
    expect(out.text).toContain('## <a id="prerequisites"></a>');
  });
});

describe('tricky document', () => {
  const src = fixture('tricky.md');
  const ex = extract(src, DNT);
  const tm = pseudoMap(ex);
  const out = assembleDocument(ex, tm, { wrap: true, preserveAnchors: true });

  it('keeps the structure', () => {
    expect(out.reverted).toEqual([]);
    expect(compareSkeletons(skeleton(src), skeleton(out.text))).toBe(-1);
  });

  it('keeps shortcut and collapsed references resolvable', () => {
    expect(out.text).toContain(`[${pseudoWords('Contoso')}][Contoso]`);
    expect(out.text).toContain(`[${pseudoWords('Fabrikam')}][Fabrikam]`);
  });

  it('keeps reference bytes when the label is not translated', () => {
    const rex = extract('See [Contoso] and [Fabrikam][] now.\n\n[Contoso]: https://contoso.com\n[Fabrikam]: https://fabrikam.com\n', []);
    const p = rex.segments[0];
    const out2 = applyTranslations(rex, new Map([[p.id, p.masked.replace('See', 'Siehe').replace('and', 'und').replace('now', 'jetzt')]]));
    expect(out2.startsWith('Siehe [Contoso] und [Fabrikam][] jetzt.')).toBe(true);
  });

  it('re-wraps soft wrapped paragraphs with container prefixes and never splits inline code', () => {
    expect(out.text).toContain('`code\nspans`');
    const quote = out.text.split('\n').filter((l) => l.startsWith('> ') && l.includes('Ü'));
    expect(quote.length).toBeGreaterThanOrEqual(3);
    expect(out.text).toContain('settings.local.json');
    expect(out.text).toContain(`  ${pseudoWords('and a second paragraph that')}`);
  });

  it('translates comments in quoted and indented code blocks, keeping code bytes', () => {
    expect(out.text).toContain(`> // ${pseudoWords('comment inside a quoted code block')}\n> const x = 1;`);
    expect(out.text).toContain(`  # ${pseudoWords('yaml comment in a list item')}\n  key: value # ${pseudoWords('trailing comment')}\n  url: "http://example.com/#anchor"`);
    expect(out.text).toContain(`/// <summary>\n/// ${pseudoWords('Gets the value of the')} <see cref="Foo"/> ${pseudoWords('property')}.`);
    expect(out.text).toContain(`/// <param name="id">${pseudoWords('The identifier of the item')}.</param>`);
    expect(out.text).toContain(` * @param items ${pseudoWords('the items in the cart')}`);
    expect(out.text).toContain(`-- ${pseudoWords('Select the active users')}\nSELECT * FROM users WHERE name = 'O''Brien -- not a comment';`);
    expect(out.text).toContain(`public int Get(int id) => id; // ${pseudoWords('returns the id')}`);
  });

  it('translates HTML table cells and summary but not tags', () => {
    expect(out.text).toContain(`<tr><td>${pseudoWords('Alpha')}</td><td>${pseudoWords('The first letter of the alphabet')}</td></tr>`);
    expect(out.text).toContain(`<summary>${pseudoWords('Click to expand the details')}</summary>`);
  });

  it('protects identifiers', () => {
    for (const id of ['Snake_case_identifier', 'camelCaseName', 'PascalCaseType', 'Get-AzContext', 'v1.2.3', '(#tricky-document)']) expect(out.text).toContain(id);
  });
});

describe('protection rules', () => {
  const protectedIn = (s: string) => findProtected(s).map((p) => s.slice(p.start, p.end));
  it.each([
    ['Open test.jpeg now', ['test.jpeg']],
    ['Edit src/app/main.ts and C:\\temp\\x.log', ['src/app/main.ts', 'C:\\temp\\x.log']],
    ['Use and/or input/output', []],
    ['Set $env:PATH and %TEMP% then run --verbose', ['$env:PATH', '%TEMP%', '--verbose']],
    ['Visit https://example.com/a?b=1.', ['https://example.com/a?b=1']],
    ['Call getValue() on System.IO.File', ['getValue()', 'System.IO.File']],
  ])('%s', (text, expected) => {
    expect(protectedIn(text)).toEqual(expected);
  });
});

describe('validation', () => {
  const ex = extract('Hello **bold** and `code` here.\n', []);
  const seg = ex.segments[0];

  it('rejects missing and duplicated tags', () => {
    expect(checkTags(seg, 'Hallo fett und hier.')).not.toEqual([]);
    expect(checkTags(seg, 'Hallo <g1>fett</g1> <x2/> <x2/>.')).not.toEqual([]);
    expect(validateSegment(seg, 'Hallo <g1>fett</g1> und <x2/> hier.', ex, LANG)).toEqual([]);
  });

  it('accepts reordered tags', () => {
    expect(validateSegment(seg, 'Hier <x2/> und <g1>fett</g1>, hallo.', ex, LANG)).toEqual([]);
  });

  it('fixes whitespace inside emphasis tags', () => {
    expect(validateSegment(seg, 'Hallo <g1> fett </g1> und <x2/> hier.', ex, LANG)).toEqual([]);
  });

  it('rejects comment terminators inside block comments', async () => {
    const cex = extract('```js\n/* explain the retry policy here */\nx();\n```\n', []);
    const c = cex.segments[0];
    expect(validateSegment(c, 'erklärt */ die Richtlinie', cex, LANG).join()).toMatch(/terminate/);
  });

  it('never lets a translated paragraph start a list', () => {
    const pex = extract('First step is easy.\n', []);
    const p = pex.segments[0];
    const out = applyTranslations(pex, new Map([[p.id, '1. Schritt ist einfach.']]));
    expect(out).toBe('1\\. Schritt ist einfach.\n');
  });

  it('wrap never starts a line with block syntax', () => {
    expect(safeLineStart('-')).toBe(false);
    expect(safeLineStart('1.')).toBe(false);
    expect(safeLineStart('#')).toBe(false);
    expect(safeLineStart('Wort')).toBe(true);
  });

  it('round-trips masked text through the NMT HTML format', () => {
    const m = 'A <g1>b <g2>c</g2></g1> <x3/> &lt;d&gt;';
    expect(htmlToMasked(maskedToHtml(m))).toBe(m);
  });

  it('quotes YAML values when needed', () => {
    const yex = extract('---\ntitle: Hello world\n---\n\nText.\n', []);
    const t = yex.segments.find((s) => s.kind === 'frontmatter')!;
    const out = applyTranslations(yex, new Map([[t.id, 'Hallo: Welt']]));
    expect(out).toContain('title: "Hallo: Welt"');
  });
});
