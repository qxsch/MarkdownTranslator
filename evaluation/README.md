# Translation quality evaluation

Run 2026-09-26 15:10 UTC, commit `85aeec5`. 68 English documents translated into de, fr. Translator: Foundry deployment `translate` (reasoning medium), review `translate` (reasoning high). Judges: `translate-alt` (foundry, openai), `translate` (foundry, openai), `copilot:claude-opus-5.5` (copilot, anthropic), `copilot:gpt-6-sol` (copilot, openai).

## Summary

- Regression status: **no regressions against d72dbf7+dirty (2026-09-26)**.
- Default configuration: 100.0% of protected strings kept verbatim, 100.0% of expected phrases translated, 100.0% of documents without any defect, 0 structure breaks, 0 changed code blocks, 0 failed documents.
- Every shipped default matches the evidence of this run.
- **review** is significantly better with the feature on: won 190, lost 1 (Holm-adjusted p < 0.001), error score 1.40 vs 29.00 per 100 words.
- **codeComments** is significantly better with the feature on: won 57, lost 5 (Holm-adjusted p < 0.001), error score 2.32 vs 32.91 per 100 words.
- **docstrings** is significantly better with the feature on: won 12, lost 1 (Holm-adjusted p 0.010), error score 1.34 vs 26.45 per 100 words.
- **frontMatter** is significantly better with the feature on: won 17, lost 0 (Holm-adjusted p < 0.001), error score 0.19 vs 44.24 per 100 words.
- **mdx (.md files)** is significantly worse with the feature on: won 2, lost 144 (Holm-adjusted p < 0.001), error score 46.09 vs 1.42 per 100 words.

## Feature decisions

Rule: a feature is recommended on when it wins the judged comparison significantly (Holm-adjusted sign test, p < 0.05) or avoids deterministic problems without a significant quality loss; otherwise off. Opposing signals are flagged for manual review. "Not exercised" means the switch changed no output in this corpus, so there is no evidence either way.

| Feature | Shipped | Evidence | Affected docs | Won / tie / lost | Win rate (95% CI) | p (Holm) | MQM on / off | Reduction (95% CI) | Problems on / off |
|---|---|---|---|---|---|---|---|---|---|
| review | on | **on** | 92/136 | 190 / 1 / 1 | 99% (97% to 100%) | < 0.001 | 1.40 / 29.00 | 27.60 (23.41 to 32.16) | 2 / 28 |
| structuralContext | off | **off** | 125/136 | 126 / 205 / 97 | 57% (50% to 63%) | 0.121 | 3.16 / 3.56 | 0.41 (-0.23 to 0.99) | 2 / 2 |
| nmtFallback | on | **not exercised** | 0/136 | 0 / 0 / 0 | n/a | n/a | n/a | n/a | 2 / 2 |
| preserveAnchors | on | **on** | 132/136 | 0 / 0 / 0 | n/a | n/a | n/a | n/a | 2 / 56 |
| codeComments | on | **on** | 30/136 | 57 / 9 / 5 | 92% (82% to 97%) | < 0.001 | 2.32 / 32.91 | 30.59 (27.76 to 33.50) | 2 / 336 |
| docstrings | on | **on** | 12/136 | 12 / 1 / 1 | 92% (67% to 99%) | 0.010 | 1.34 / 26.45 | 25.11 (20.32 to 29.42) | 2 / 150 |
| frontMatter | on | **on** | 17/136 | 17 / 3 / 0 | 100% (82% to 100%) | < 0.001 | 0.19 / 44.24 | 44.06 (38.70 to 52.61) | 2 / 72 |
| mdx (.md files) | off | **off** | 22/130 | 2 / 0 / 144 | 1% (0% to 5%) | < 0.001 | 46.09 / 1.42 | -44.67 (-50.02 to -40.48) | 304 / 2 |
| mathSingleDollar | off | **not exercised** | 0/136 | 0 / 0 / 0 | n/a | n/a | n/a | n/a | 2 / 2 |
| mdx (.mdx files) | on | **on** | 4/6 | 6 / 8 / 3 | 67% (35% to 88%) | 0.508 | 6.71 / 14.94 | 8.23 (-5.03 to 23.80) | 0 / 16 |

## 1. Judged translation quality

Every block (paragraph, list, table, code block, front matter) whose output differs between the feature on and off is judged blind: each judge sees the English source and both candidates as A and B, twice with the order swapped. A judge's verdict counts only when both orders agree; the panel verdict is the majority of the judges. Error annotations follow MQM (minor 1, major 5, critical 10 points), normalised per 100 source words.

![Error reduction with confidence intervals](images/quality-mqm-reduction.svg)

![Pairwise preference](images/quality-preference.svg)

Judges from a different model family than the translator (`copilot:claude-opus-5.5`) guard against self-preference:

| Feature | Independent judges: won / tie / lost | p (unadjusted) |
|---|---|---|
| review | 168 / 22 / 1 | < 0.001 |
| structuralContext | 51 / 335 / 42 | 0.407 |
| codeComments | 57 / 12 / 2 | < 0.001 |
| docstrings | 13 / 1 / 0 | < 0.001 |
| frontMatter | 17 / 3 / 0 | < 0.001 |
| mdx (.md files) | 1 / 2 / 143 | < 0.001 |
| mdx (.mdx files) | 6 / 9 / 2 | 0.289 |

Per judge (won / lost with the feature on, sign test):

| Feature | `translate-alt` | `translate` | `copilot:claude-opus-5.5` | `copilot:gpt-6-sol` |
|---|---|---|---|---|
| review | 186 / 0 (p < 0.001) | 187 / 0 (p < 0.001) | 168 / 1 (p < 0.001) | 183 / 2 (p < 0.001) |
| structuralContext | 83 / 60 (p 0.065) | 96 / 72 (p 0.076) | 51 / 42 (p 0.407) | 63 / 63 (p 1.000) |
| codeComments | 57 / 5 (p < 0.001) | 57 / 3 (p < 0.001) | 57 / 2 (p < 0.001) | 56 / 5 (p < 0.001) |
| docstrings | 12 / 1 (p 0.003) | 12 / 2 (p 0.013) | 13 / 0 (p < 0.001) | 12 / 1 (p 0.003) |
| frontMatter | 17 / 0 (p < 0.001) | 17 / 0 (p < 0.001) | 17 / 0 (p < 0.001) | 17 / 0 (p < 0.001) |
| mdx (.md files) | 2 / 142 (p < 0.001) | 2 / 144 (p < 0.001) | 1 / 143 (p < 0.001) | 1 / 144 (p < 0.001) |
| mdx (.mdx files) | 6 / 2 (p 0.289) | 6 / 3 (p 0.508) | 6 / 2 (p 0.289) | 6 / 3 (p 0.508) |

## 2. Error profile

![Error categories](images/quality-error-categories.svg)

## 3. Fidelity and correctness (deterministic)

Checked on every output without a model: expectations in [eval/features/expect.json](../eval/features/expect.json) (strings that must stay byte-identical, phrases that must be translated), Markdown structure re-parsed and compared node by node, code blocks compared after stripping comments and docstrings, in-page links resolved against the translated headings and anchors, and source segments that come back unchanged (English left behind).

![Deterministic problems](images/fidelity-problems.svg)

| Feature | Protected changed on / off | Not translated on / off | Structure on / off | Code on / off | Links on / off | English segments on / off | Failures on / off | Tokens on / off |
|---|---|---|---|---|---|---|---|---|
| review | 0 / 0 | 0 / 12 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 16 | 0 / 0 | 805561 / 375568 |
| structuralContext | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 2 | 0 / 0 | 877160 / 805561 |
| nmtFallback | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 2 | 0 / 0 | 805561 / 805561 |
| preserveAnchors | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 54 | 2 / 2 | 0 / 0 | 805561 / 805561 |
| codeComments | 0 / 0 | 0 / 140 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 196 | 0 / 0 | 805561 / 728884 |
| docstrings | 0 / 0 | 0 / 58 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 92 | 0 / 0 | 805561 / 741294 |
| frontMatter | 0 / 0 | 0 / 36 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 36 | 0 / 0 | 805561 / 753538 |
| mdx (.md files) | 0 / 0 | 110 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 176 / 2 | 18 / 0 | 646734 / 779041 |
| mathSingleDollar | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 2 / 2 | 0 / 0 | 790491 / 805561 |
| mdx (.mdx files) | 0 / 2 | 0 / 6 | 0 / 2 | 0 / 0 | 0 / 0 | 0 / 6 | 0 / 0 | 26520 / 31811 |

![Problems avoided by document type](images/breakdown-document-types.svg)

## 4. Default configuration scorecard

| Metric | Value |
|---|---|
| Documents x languages | 136 |
| Protected strings kept verbatim | 100.0% of 922 |
| Expected phrases translated | 100.0% of 686 |
| Documents without defects | 100.0% |
| Structure breaks / changed code blocks / broken links | 0 / 0 / 0 |
| Failed documents | 0 |
| Blocks reverted by the structure check | 0 |
| Segments kept in English after validation | 0 |
| Segments translated by the NMT fallback | 0 |
| English segments left (attribute 2) | 2 |
| Tokens per document (mean) | 5923 |
| Seconds per document (median / p90) | 130.9 / 197.3 |

![Pass rate by document type](images/default-config-by-document-type.svg)

## 5. Breakdown by language

![By language](images/breakdown-languages.svg)

## 6. Judge reliability

| Judge | Provider | Family | Judgements | Failed | Decisive | Position consistency | Agreement with other judges | Cohen’s kappa vs others |
|---|---|---|---|---|---|---|---|---|
| `translate-alt` | foundry | openai | 888 | 0 | 65% | 97% | 87% | 0.80 |
| `translate` | foundry | openai | 888 | 0 | 68% | 98% | 88% | 0.82 |
| `copilot:claude-opus-5.5` | copilot | anthropic | 887 | 1 | 57% | 99% | 80% | 0.70 |
| `copilot:gpt-6-sol` | copilot | openai | 888 | 0 | 63% | 99% | 86% | 0.78 |

Position consistency is the share of decisive judgements where both presentation orders picked the same candidate. Fleiss’ kappa over all judges: 0.79 (0.2 to 0.4 fair, 0.4 to 0.6 moderate, above 0.6 substantial).

![Judge agreement](images/judge-agreement.svg)

## 7. Cost

![Tokens per document](images/cost-tokens.svg)

## 8. Regression report

Compared with run 2026-09-26 14:08 UTC, commit `d72dbf7+dirty` (de, fr, 68 documents).

| Area | Metric | Before | After | Status |
|---|---|---|---|---|
| default configuration | protected content kept | 98.3% | 100.0% | improvement |
| default configuration | documents without defects | 90.4% | 100.0% | improvement |
| default configuration | English segments left | 4 | 2 | improvement |
| cost | tokens per document | 6699 | 5923 | improvement |
| mathSingleDollar | recommendation | off | not exercised | changed |

New defects: 0. Fixed defects: 3. No longer checked because the expectation was changed: 15.

Fixed defects:

- de code-no-lang.md: untranslated `Show the version`
- fr code-no-lang.md: untranslated `Show the version`
- fr docstrings-numpy.md: overtranslated `quota_report : `

No longer checked (expectation changed):

- de blog-informal.md: overtranslated `200,000`
- fr blog-informal.md: overtranslated `200,000`
- de escapes-entities.md: overtranslated `\'backticks\'`
- de escapes-entities.md: overtranslated `\[brackets\]`
- de escapes-entities.md: overtranslated `&lt;tag&gt;`
- fr escapes-entities.md: overtranslated `\'backticks\'`
- fr escapes-entities.md: overtranslated `\[brackets\]`
- fr escapes-entities.md: overtranslated `&lt;tag&gt;`
- de filenames-paths.md: overtranslated `settings.json and restart`
- fr filenames-paths.md: overtranslated `settings.json and restart`
- de tables-complex.md: overtranslated `[Audit log](#audit-log)`
- fr tables-complex.md: overtranslated `[Audit log](#audit-log)`
- de ui-labels-table.md: overtranslated `Shift+F5`
- de ui-labels-table.md: overtranslated `Ctrl+B`
- fr ui-labels-table.md: overtranslated `Shift+F5`

## Method

Dimensions measured:

1. **Judged quality**: blind pairwise preference with position swap, panel majority, exact sign test with Holm correction over all features, Wilson 95% interval of the win rate, MQM error score with a paired bootstrap interval (2000 resamples over judged blocks, fixed seed).
2. **Error profile**: MQM categories (mistranslation, omission, terminology, fluency, register, untranslated, overtranslation, markup, ...) weighted by severity.
3. **Fidelity**: deterministic checks that need no model: protected strings, expected translations, Markdown structure, code bytes, in-page links, English left behind, failures.
4. **Robustness**: validation retries, blocks reverted by the structure check, segments kept in English, NMT fallbacks.
5. **Cost**: tokens and wall-clock time per document.
6. **Judge reliability**: position consistency, pairwise Cohen’s kappa, Fleiss’ kappa, per-judge results and a subset of judges from another model family than the translator.
7. **Regression**: every run stores a snapshot in `history/`; the report compares with the previous snapshot and lists new and fixed defects.

Isolation: each feature is flipped alone against the shipped defaults. Segments that the switch does not change are reused from the default run, so differences come from the feature and not from sampling noise. Review off, NMT fallback off and anchors off are derived exactly from the default run.

Limitations: LLM judges instead of professional linguists; judge families overlap with the translator unless independent judges are configured; the corpus is synthetic technical documentation; "affected" features with few differing blocks have wide intervals.

## Reproduce

```powershell
npm install; npm install --prefix eval             # evaluator dependencies (host only, not in the container)
$env:EVAL_LANGS = 'de,fr'
$env:EVAL_JUDGES = 'translate-alt,translate,copilot:claude-opus-5.5,copilot:gpt-6-sol'
npx tsx eval/features.ts                            # translate, check, judge, then write this report
npx tsx eval/report.ts                              # rebuild the report from the latest results
```

See the [README](../README.md#evaluation-and-tests) for all options.

## Data

- [data/feature-summary.json](data/feature-summary.json): every number in this report
- [data/judgements.csv](data/judgements.csv): one row per judged block and judge
- [data/records.csv](data/records.csv): deterministic checks per document, language and feature
- [data/defects.csv](data/defects.csv): defects of the default configuration
- [translations/](translations/): the translated corpus (default configuration) next to the sources in [eval/features/](../eval/features/)
- [history/](history/): snapshots for regression tracking
