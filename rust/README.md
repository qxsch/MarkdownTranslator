# mdtranslate (Rust)

A self-contained, statically linked command-line translator for Markdown and MDX. It is a port of the
TypeScript implementation in [`../src`](../src). The two produce **byte-identical** extractions and
assembled documents on the whole test corpus (227 golden files) and on 1,313 pages of public documentation.
It uses the same prompts, the same Azure AI Foundry and Azure Translator calls, and the same `MDT_*` feature
switches.

- **Linux x64**: `x86_64-unknown-linux-musl`, fully static (no glibc, no OpenSSL; TLS via rustls).
- **Windows x64**: `x86_64-pc-windows-msvc` with the static C runtime; only system DLLs (kernel32 and similar).
- One file, about 27 MB (15 tree-sitter grammars and the JavaScript parser for MDX are compiled in).

## Usage

```powershell
mdtranslate -sourceFile docs/guide.md -targetFile docs/guide.de.md -lang de
cat guide.md | mdtranslate -lang fr > guide.fr.md
mdtranslate -sourceFile guide.md -targetFile "out/guide.{lang}.md" -lang de,fr,sv -reportFile out/report.json
mdtranslate -sourceFile guide.md -lang de -engine pseudo                    # offline, deterministic
```

Without `-sourceFile` the document is read from stdin, and without `-targetFile` the result goes to stdout;
`-` means the same explicitly. The two ways of reading stdin differ when there is no document:

```bash
cat /dev/null | mdtranslate -lang fr                  # usage on stderr, exit code 1
cat /dev/null | mdtranslate -sourceFile - -lang fr    # empty output, exit code 0
```

- Without `-sourceFile`, stdin has to bring a document. When nothing is piped in (an interactive terminal) or
  the input is empty or only whitespace, `mdtranslate` prints the usage to stderr and exits with code 1: the
  options were probably forgotten.
- An explicit source may be empty: `-sourceFile empty.md`, or `-sourceFile -` with empty input, writes the
  input unchanged (an empty translation) without contacting any service, exit code 0. `-sourceFile -` reads
  stdin even from a terminal (end the input with Ctrl+D, or Ctrl+Z and Enter on Windows).
- A source file that does not exist is an error (exit code 1), and no target is written.

**stdout carries nothing but the output** (the translation, or the JSON of an inspection mode below) and
`-help`. Progress, warnings and errors go to stderr, and errors are printed even with `-quiet`.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | usage, configuration or input error (bad option, no input, unreadable file, invalid MDX, unknown language, no service configured) |
| 2 | translation failed and the **source was written unchanged**: the structure check failed, or no segment could be translated (for example the Azure service was unreachable or rejected the credentials). Also used when `-analyzeOnly` fails. |
| 3 | partially translated: some segments were kept in the source language after retries and the NMT fallback (details on stderr and in `-reportFile`) |

### Options

| Option | Purpose |
|---|---|
| `-sourceFile <file\|->` | Markdown/MDX input (default and `-`: stdin) |
| `-targetFile <file\|->` | output (default and `-`: stdout); a file name containing `{lang}` with several languages |
| `-lang <codes>` | target languages, comma-separated (`-listLanguages` shows the catalog) |
| `-fileName <name>` | logical file name for stdin input (`.mdx` detection, prompts) |
| `-engine gpt\|nmt\|pseudo` | model, Azure Translator only, or the offline pseudo translator (`MDT_ENGINE`) |
| `-sourceLanguage <code>` | source language (`MDT_SOURCE_LANGUAGE`; default: detected, else `en`) |
| `-formality formal\|informal` | register (`MDT_FORMALITY`; default: front matter `formality:`, else detected) |
| `-doNotTranslate <a,b>` | extra protected terms (added to the glossary's list) |
| `-translateDeployment`, `-reviewDeployment`, `-analysisDeployment` | Foundry deployments (`MDT_*_DEPLOYMENT`) |
| `-reportFile <file>` | JSON report: per-language report, outcomes, translation memory, token usage |
| `-reportSegments` | include each segment's source and rendered translation in the report |
| `-analysisFile <file>` | use a saved document analysis instead of asking the model |
| `-tmFile <file>` | assemble from a saved translation memory (`{"tm": [[id, text]...], "outcomes": [...]}`); no model calls |
| `-reviewTm` | with `-tmFile`: run the review pass over that translation memory |
| `-cacheDir <dir>` | segment cache (`MDT_CACHE_DIR`); the file format and keys match the TypeScript cache |
| `-maxConcurrency <n>` | parallel Azure requests (`MDT_MAX_CONCURRENCY`, default 16) |
| `-languagesFile`, `-glossaryFile` | override the built-in `config/languages.json` / `config/glossary.json` (`MDT_LANGUAGES_FILE`, `MDT_GLOSSARY_FILE`) |
| `-envFile <file>` | read variables from a `.env` file, for the configuration and the credential chain alike (real environment variables win) |
| `-dumpExtraction`, `-dumpGolden`, `-analyzeOnly`, `-listLanguages` | inspection output as JSON to `-targetFile` or stdout |
| `-quiet`, `-help`, `-version` | `-version` prints to stderr |

Options are case-insensitive and accept `-name value`, `-name=value`, `--name` and kebab-case
(`--source-file`).

### Feature flags

Each flag takes `-flag`, `-no-flag` or `-flag=true|false`. Without one, the environment variable decides,
exactly as in the TypeScript service.

| Flag | Environment | Default | Effect |
|---|---|---|---|
| `-review` | `MDT_REVIEW` | on | second pass by a reviewer model |
| `-structuralContext` | `MDT_STRUCTURAL_CONTEXT` | off | send each segment's position (section, table column/row, list lead-in) |
| `-nmtFallback` | `MDT_NMT_FALLBACK` | on | Azure Translator for segments that keep failing validation |
| `-preserveAnchors` | `MDT_PRESERVE_ANCHORS` | on | keep links to the original heading anchors working |
| `-docstrings` | `MDT_DOCSTRINGS` | on | translate Python docstrings (Google/NumPy/reST structure kept) |
| `-codeComments` | `MDT_CODE_COMMENTS` | on | translate comments in fenced code blocks |
| `-frontMatter` | `MDT_FRONT_MATTER` | on | translate prose values in YAML front matter |
| `-mathSingleDollar` | `MDT_MATH_SINGLE_DOLLAR` | off | parse `$x$` as inline math |
| `-mdx` | `MDT_MDX` | `.mdx` files | parse as MDX |

Other environment variables are the same as for the TypeScript service (see [`.env.example`](../.env.example)).
They include `AZURE_OPENAI_ENDPOINT`, `AZURE_TRANSLATOR_ENDPOINT`, `AZURE_TRANSLATOR_REGION`, `AZURE_AI_API_KEY`,
`MDT_TRANSLATE_REASONING`, `MDT_REVIEW_REASONING`, `MDT_BATCH_MAX_SEGMENTS`, `MDT_BATCH_MAX_CHARS`,
`MDT_CONTEXT_MAX_CHARS` and `MDT_REQUEST_TIMEOUT_MS`. Two are new here: `MDT_MAX_ATTEMPTS` (attempts per
Azure request, default 6) and `MDT_ENGINE`.

### Authentication

With `AZURE_AI_API_KEY`, API keys are used for both services. Otherwise Microsoft Entra ID is used. Tokens
are cached and refreshed five minutes before they expire. The credential chain is:

1. The user-assigned managed identity, when `AZURE_CLIENT_ID` is set.
2. Then the `DefaultAzureCredential` order:
   - service principal (`AZURE_TENANT_ID` / `AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET`)
   - workload identity (`AZURE_FEDERATED_TOKEN_FILE`)
   - managed identity: App Service, Container Apps and Functions (`IDENTITY_ENDPOINT`), Cloud Shell, or IMDS
   - Azure CLI
   - Azure PowerShell (`pwsh`, then Windows PowerShell; Az.Accounts 2.2 or later)
   - Azure Developer CLI

`AZURE_AI_ACCESS_TOKEN` passes a pre-acquired token. As in `@azure/identity`, the command-line tools run in
`%SystemRoot%` (`/bin` elsewhere), so an `az` placed in the working directory is never picked up. Tenant ids
must consist of letters, digits, `-` and `.`. Requests that are waiting while an acquisition fails get the
same error instead of running the chain again one by one; the next retry round tries again.

## Building

```powershell
cargo build --release                                   # host platform
cargo test --release                                    # unit tests, CLI contract, golden files
```

Static release binaries (what [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) builds and verifies):

```bash
# Windows x64 (static CRT is set in .cargo/config.toml)
cargo build --release --target x86_64-pc-windows-msvc

# Linux x64, fully static: zig compiles the C parts (tree-sitter grammars, mimalloc) against musl.
# Any zig on PATH works as well (for example the zip from ziglang.org); Windows hosts can cross-build too.
pip install ziglang==0.16.0 && cargo install cargo-zigbuild --locked --version 0.23.4
rustup target add x86_64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

The musl build uses mimalloc, because musl's allocator is slow under contention. Without the default
`grammars` feature, the tree-sitter grammars are left out. That gives a smaller binary, but comments are
then detected only for the lexer languages.

## Parity with the TypeScript implementation

The TypeScript code stays the reference.
[`tests/golden`](tests/golden) holds its output for every fixture in `eval/features`, `eval/corpus`,
`test/fixtures` and [`tests/fixtures`](tests/fixtures) (regression inputs for the port): extracted segments,
masked text, hints, wrap specs, inline signatures, replacement offsets (bytes), and the assembled pseudo
translation. Each fixture has the default switches, plus each switch flipped where that changes anything.
`cargo test` must reproduce all of it byte for byte.

`golden.ts --check --dir` also finds identical results on 1,313 of 1,313 real-world pages:

| Source | Pages |
|---|---|
| MicrosoftDocs/azure-docs `articles/storage/blobs` | 334 |
| github/docs `content/actions` | 247 |
| facebook/docusaurus `website/docs` (MDX) | 94 |
| MicrosoftDocs/PowerShell-Docs `reference/docs-conceptual` | 638 |

| Tool | Purpose |
|---|---|
| `npx tsx rust/tools/golden.ts` | regenerate the golden files from the TypeScript code |
| `npx tsx rust/tools/golden.ts --check [--dir <docs>]` | compare both implementations live on any Markdown folder |
| `npx tsx rust/tools/mdast-dump.ts <out> <files/dirs>` + `cargo run --example probe <file>` | compare the Markdown syntax trees |
| `npx tsx rust/tools/html-dump.ts <out> <files>` + `cargo run --example probe --html <file>` | compare HTML trees with parse5 |
| `npx tsx rust/tools/mock-azure.ts [port]` | local stand-in for Azure OpenAI, Translator and the Entra token endpoint (throttling, broken answers, review edits, `MOCK_TOKEN`) |

How the port matches the JavaScript libraries:

- **Markdown**: markdown-rs (a micromark port), corrected where its tree differs from mdast-util-from-markdown:
  - character escapes at the start of text
  - reference identifiers
  - list end positions
  - GFM literal autolinks, which are only found where micromark's syntax extension finds them (the TypeScript
    code drops the mdast transform)
  - GFM table body rows without a leading pipe that start with `#`, `` ` ``, `~`, `$`, `*`, `_`, `e`, `i` or
    `{`: markdown-rs ended the table there. This is fixed in a vendored copy
    ([`vendor/markdown`](vendor/markdown/README.md), a five-line patch to 1.0.0).
- **Block directives** (`:::note[Title]{.cls}` containers and `::leaf[...]` leaves) are not in markdown-rs.
  They are parsed in two passes: fences are found with micromark-extension-directive's rules, then replaced by
  same-length thematic breaks, and the tree is regrouped.
- **HTML**: a parse5-compatible fragment parser with source locations. It covers implied end tags, raw-text
  elements, tables (implicit `tbody`, foster parenting), active formatting reconstruction and the adoption
  agency algorithm, and it matches parse5 on every probe.
- **MDX**: expressions and ESM are validated with the oxc JavaScript parser, where the TypeScript code uses
  acorn, so invalid MDX is rejected the same way.
- **YAML**: saphyr-parser events give scalar styles and byte ranges (like `yaml`'s `node.range`) with YAML 1.2
  core schema typing.
- **Code**: native tree-sitter grammars, pinned to the versions in `@vscode/tree-sitter-wasm` where known.
- **Regular expressions**: the 97 literals use JavaScript semantics. `\w`, `\d` and `\b` are ASCII, `\s` and
  `.` follow JavaScript, and lookbehinds go through fancy-regex. A test compiles every literal.
- **Widths and limits**: widths, columns and hint lengths are counted in UTF-16 code units like JavaScript
  strings, so re-wrapped output matches.
- **Slugs**: github-slugger's table is generated from its regex (`tools/gen-slug-table.ts`).

Known, deliberate differences:

- Error messages (YAML, MDX) differ in wording; the failing stage is the same.
- A hint longer than 80 characters is cut without splitting a surrogate pair.
- Source language names for the prompts come from a built-in table instead of `Intl.DisplayNames`.
- A reference definition placed inside an unclosed container directive at the very end of a document
  resolves in directive labels; in micromark it does not.
- The HTTP API (`src/server.ts`) is not part of the binary; the binary covers the command-line use.

## Evaluation

The evaluation framework stays in TypeScript ([`eval`](../eval)), but the implementation under test is this
binary. `eval/rust.ts` runs it for translation, assembly of derived variants (`-tmFile`), extraction
fingerprints (`-dumpExtraction`) and document analysis (`-analyzeOnly`). The TypeScript code only provides the
independent checks and the Foundry judge client.

```powershell
cargo build --release --manifest-path rust/Cargo.toml
npx tsx eval/features.ts          # MDT_RUST_BIN=<path> to test another build; EVAL_PARALLEL processes (default 8)
```

## Layout

| Rust | TypeScript |
|---|---|
| `markdown/parse.rs`, `ast.rs`, `directive.rs`, `mdx.rs` | `markdown/parse.ts` |
| `markdown/extract.rs`, `context.rs`, `inline.rs` | `markdown/extract.ts`, `context.ts`, `inline.ts` |
| `markdown/html.rs`, `html_parser.rs`, `html_attrs.rs` | `markdown/html.ts` (parse5), `htmlAttrs.ts` |
| `markdown/frontmatter.rs`, `yaml.rs` | `markdown/frontmatter.ts` (yaml) |
| `markdown/render.rs`, `wrap.rs`, `document.rs`, `anchors.rs`, `line_map.rs`, `math_spans.rs` | same names |
| `mask/masking.rs`, `protect.rs` | `mask/masking.ts`, `protect.ts` |
| `code/parsers.rs`, `comments.rs` | `code/parsers.ts`, `comments.ts` |
| `translate/engine.rs`, `prompts.rs`, `validate.rs`, `cache.rs` | `translate/*` |
| `azure/http.rs`, `auth.rs`, `clients.rs` | `azure/http.ts`, `openai.ts`, `nmt.ts` |
| `assemble.rs`, `pipeline.rs`, `config.rs`, `main.rs` | `assemble.ts`, `pipeline.ts`, `config.ts`, `scripts/translate.ts` |
