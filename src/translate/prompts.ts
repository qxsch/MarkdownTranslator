import type { Glossary, LanguageConfig } from '../config.js';
import type { Segment } from '../markdown/types.js';

export const PROMPT_VERSION = 'v3';

export interface DocAnalysis {
  sourceLanguage: string;
  register: 'formal' | 'informal' | 'neutral';
  registerEvidence: string;
  domain: string;
  audience: string;
  summary: string;
  doNotTranslate: string[];
  terminology: { term: string; note: string }[];
}

export const ANALYSIS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['sourceLanguage', 'register', 'registerEvidence', 'domain', 'audience', 'summary', 'doNotTranslate', 'terminology'],
  properties: {
    sourceLanguage: { type: 'string', description: 'BCP-47 code of the document language, e.g. en' },
    register: { type: 'string', enum: ['formal', 'informal', 'neutral'] },
    registerEvidence: { type: 'string' },
    domain: { type: 'string' },
    audience: { type: 'string' },
    summary: { type: 'string' },
    doNotTranslate: { type: 'array', items: { type: 'string' } },
    terminology: {
      type: 'array',
      items: { type: 'object', additionalProperties: false, required: ['term', 'note'], properties: { term: { type: 'string' }, note: { type: 'string' } } },
    },
  },
} as const;

export const ANALYSIS_SYSTEM = `You analyze a Markdown document before it is localized.
Determine:
- sourceLanguage: the BCP-47 language code of the prose (ignore code blocks).
- register: how the author addresses the reader. "informal" only for clearly casual/chatty writing (slang, jokes, emojis, "hey", "awesome", heavy contractions); "formal" for official/legal/enterprise tone; otherwise "neutral".
- registerEvidence: one short sentence quoting the evidence.
- domain and audience: short phrases.
- summary: 2-4 sentences describing the content, for translator context.
- doNotTranslate: product names, brand names, UI labels in code font, API/feature names that must stay in the source language. Only names, never common words.
- terminology: up to 25 domain terms whose translation must be consistent, with a short note on meaning in this document.`;

export const TRANSLATION_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['translations'],
  properties: {
    translations: {
      type: 'array',
      items: { type: 'object', additionalProperties: false, required: ['id', 'text'], properties: { id: { type: 'string' }, text: { type: 'string' } } },
    },
  },
} as const;

export const REVIEW_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['edits'],
  properties: {
    edits: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['id', 'category', 'severity', 'explanation', 'text'],
        properties: {
          id: { type: 'string' },
          category: { type: 'string', enum: ['accuracy', 'omission', 'addition', 'terminology', 'grammar', 'spelling', 'fluency', 'register', 'locale', 'consistency', 'untranslated'] },
          severity: { type: 'string', enum: ['minor', 'major', 'critical'] },
          explanation: { type: 'string' },
          text: { type: 'string' },
        },
      },
    },
  },
} as const;

const TAG_RULES = `Segment format:
- Segments contain inline markup tags. <gN>…</gN> wraps formatted text (bold, italic, link text, HTML element content). <xN/> stands for content that must not change (code, file names, paths, URLs, variables, line breaks, escapes).
- Keep every tag exactly once, with the same number. You may move tags and reorder words so the sentence is natural in the target language; the text inside <gN>…</gN> must be the translation of the source text inside that same pair.
- The "tags" map shows what each tag stands for. Use it only for grammar (gender, case, articles, prepositions); never copy or translate its content into the text.
- Output plain text plus the tags. Do not add Markdown, HTML, backticks, asterisks, underscores, brackets or new line breaks. Keep &lt; &gt; &amp; as written.`;

function formalityGuidance(lang: LanguageConfig, formality: 'formal' | 'informal'): string {
  return formality === 'informal' ? lang.informal : lang.formal;
}

function glossaryFor(glossary: Glossary, lang: LanguageConfig): string {
  const lines: string[] = [];
  for (const [term, byLang] of Object.entries(glossary.terms)) {
    const t = byLang[lang.code] ?? byLang[lang.code.split('-')[0]];
    if (t) lines.push(`- "${term}" → "${t}"`);
  }
  return lines.join('\n');
}

export function translationSystem(p: {
  source: string;
  lang: LanguageConfig;
  formality: 'formal' | 'informal';
  analysis?: DocAnalysis;
  glossary: Glossary;
}): string {
  const terms = glossaryFor(p.glossary, p.lang);
  const dnt = [...new Set([...(p.analysis?.doNotTranslate ?? [])])].slice(0, 80);
  return `You are an expert technical translator and native-level ${p.lang.name} localizer. Translate from ${p.source} into ${p.lang.name}.

Quality bar: publication-ready documentation that reads as if originally written in ${p.lang.name}. Preserve the exact meaning: no omissions, additions, explanations or summaries. Keep numbers and units correct. Never change version numbers, build numbers or identifiers ("Docker 4.30" stays "4.30"); apply locale decimal separators only to genuine measured quantities. Use established ${p.lang.name} technical terminology (as used in Microsoft and major vendor documentation for this locale) and keep terms consistent across all segments.

Register: ${formalityGuidance(p.lang, p.formality)}
Locale style: ${p.lang.style ?? 'Follow the typographic conventions of the locale.'}

${TAG_RULES}

Segment kinds:
- heading: concise title style, natural capitalization rules of ${p.lang.name}.
- cell: table cell, keep it short.
- comment: a source code comment; keep it terse and technical, keep identifiers as-is. If a comment is actually commented-out code, return it unchanged.
- alt / attr / title: image descriptions, tooltips and HTML attributes; translate them.
- frontmatter: document metadata such as title or description.
- If a segment is already in ${p.lang.name}, or consists only of names/identifiers, return it unchanged.

Keep these names in their original form: ${dnt.length ? dnt.join(', ') : '(none identified)'}.
${terms ? `Required terminology:\n${terms}\n` : ''}${p.analysis?.terminology.length ? `Key terms of this document (translate consistently):\n${p.analysis.terminology.map((t) => `- ${t.term}: ${t.note}`).join('\n')}\n` : ''}
Return JSON {"translations":[{"id":"…","text":"…"}]} with exactly one entry per input segment id.`;
}

export function segmentPayload(seg: Segment) {
  const tags: Record<string, string> = {};
  for (const p of seg.placeholders.values()) tags[`x${p.n}`] = p.hint;
  for (const p of seg.pairs.values()) tags[`g${p.n}`] = `${p.kind}: ${p.hint}`;
  return { id: seg.id, kind: seg.kind, context: seg.note, text: seg.masked, ...(Object.keys(tags).length ? { tags } : {}) };
}

export function documentContext(p: { analysis?: DocAnalysis; docName: string; source: string; maxChars: number }): string {
  const doc = p.source.length > p.maxChars ? p.source.slice(0, p.maxChars) + '\n[…truncated…]' : p.source;
  return `Document "${p.docName}"${p.analysis ? `\nDomain: ${p.analysis.domain}\nAudience: ${p.analysis.audience}\nSummary: ${p.analysis.summary}` : ''}

Full source document, for context only (do not translate it here):
<document>
${doc}
</document>`;
}

export function reviewSystem(p: { source: string; lang: LanguageConfig; formality: 'formal' | 'informal'; glossary: Glossary; analysis?: DocAnalysis }): string {
  const terms = glossaryFor(p.glossary, p.lang);
  return `You are a senior ${p.lang.name} localization reviewer (native speaker) performing a quality check of translations from ${p.source}.

For every segment compare the translation with the source and look for: mistranslations and meaning shifts, omissions, additions, wrong or inconsistent terminology, grammar, spelling and agreement errors, unnatural or literal phrasing, wrong register, locale convention errors, untranslated text.
Register: ${formalityGuidance(p.lang, p.formality)}
Locale style: ${p.lang.style ?? ''}
${terms ? `Required terminology:\n${terms}\n` : ''}
${TAG_RULES}

Only report segments that genuinely need a change. Do not make stylistic or preferential edits to translations that are already correct and natural. For each reported segment return the complete corrected translation in "text" (with all tags), the error category, severity and a one-line explanation.
Return JSON {"edits":[…]}; an empty list when everything is correct.`;
}
