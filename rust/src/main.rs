//! `mdtranslate`: structure-preserving Markdown translation from the command line.

use mdtranslate::config::{load_glossary, load_languages, parse_bool, parse_formality, AppConfig, Engine};
use mdtranslate::dump::{dump_extraction, golden, Variant};
use mdtranslate::markdown::render::render_segment;
use mdtranslate::pipeline::{MarkdownTranslator, Memory, Status, TranslateOptions};
use mdtranslate::translate::engine::SegmentOutcome;
use mdtranslate::translate::prompts::DocAnalysis;
use mdtranslate::types::{RenderOptions, TMap};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::ExitCode;
use std::time::Instant;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const USAGE: &str = "mdtranslate - structure-preserving Markdown translation (Azure AI Foundry / Azure Translator)

USAGE:
  mdtranslate -sourceFile <file|-> -targetFile <file|-> -lang <code>[,<code>...] [options]

  stdout carries nothing but the output of -targetFile - (and -help); progress and errors go to stderr.

INPUT / OUTPUT
  -sourceFile, -s <file>      Markdown or MDX file to translate; - reads stdin (required)
  -targetFile, -t <file>      where to write the translation; - writes stdout (required).
                              With several languages the name must contain {lang}.
  -lang, -l <codes>           target language code(s), comma-separated (see -listLanguages)
  -fileName <name>            logical file name for stdin input (MDX detection, prompts)
  -reportFile <file>          write a JSON report (reports, outcomes, translation memory, usage)
  -reportSegments             include per-segment source and rendered translation in the report

TRANSLATION
  -engine gpt|nmt|pseudo      model translation, Azure Translator only, or the offline pseudo translator
                              (MDT_ENGINE, default gpt)
  -sourceLanguage <code>      source language (MDT_SOURCE_LANGUAGE, default: detected, else en)
  -formality formal|informal  register (MDT_FORMALITY, default: front matter, else detected)
  -doNotTranslate <a,b,...>   extra terms that are never translated
  -translateDeployment <name> (MDT_TRANSLATE_DEPLOYMENT, default translate)
  -reviewDeployment <name>    (MDT_REVIEW_DEPLOYMENT, default: translate deployment)
  -analysisDeployment <name>  (MDT_ANALYSIS_DEPLOYMENT, default: translate deployment)
  -analysisFile <file>        use this document analysis (JSON) instead of asking the model
  -tmFile <file>              assemble from a saved translation memory ({\"tm\": [[id, text]...]}); no model calls
  -reviewTm                   with -tmFile: run the review pass over that translation memory
  -cacheDir <dir>             segment cache directory (MDT_CACHE_DIR)
  -maxConcurrency <n>         parallel Azure requests (MDT_MAX_CONCURRENCY, default 16)
  -languagesFile <file>       language catalog (MDT_LANGUAGES_FILE, default: built in)
  -glossaryFile <file>        glossary (MDT_GLOSSARY_FILE, default: built in)
  -envFile <file>             read variables from a .env file (the environment takes precedence)

FEATURE FLAGS  (-flag, -no-flag, -flag=true|false; defaults from the environment variable)
  -review            second review pass by a reviewer model      MDT_REVIEW              (on)
  -structuralContext send section/table/list position to model   MDT_STRUCTURAL_CONTEXT  (off)
  -nmtFallback       Azure Translator for segments that fail     MDT_NMT_FALLBACK        (on)
  -preserveAnchors   keep links to original heading anchors      MDT_PRESERVE_ANCHORS    (on)
  -docstrings        translate Python docstrings                 MDT_DOCSTRINGS          (on)
  -codeComments      translate comments in code blocks           MDT_CODE_COMMENTS       (on)
  -frontMatter       translate prose values in YAML front matter MDT_FRONT_MATTER        (on)
  -mathSingleDollar  parse $x$ as inline math                    MDT_MATH_SINGLE_DOLLAR  (off)
  -mdx               parse as MDX                                MDT_MDX                 (.mdx files)

INSPECTION  (JSON written to -targetFile, which may be -)
  -dumpExtraction    the extracted segments
  -dumpGolden        extraction + pseudo translation (parity tests)
  -analyzeOnly       the document analysis
  -listLanguages     the language catalog (no -sourceFile needed)
  -quiet             no progress output on stderr
  -help              this text (stdout)
  -version           version and prompt version (stderr)

AZURE (environment)
  AZURE_OPENAI_ENDPOINT, AZURE_TRANSLATOR_ENDPOINT, AZURE_TRANSLATOR_REGION, AZURE_AI_API_KEY (else Microsoft
  Entra ID: managed identity, workload identity, service principal, Azure CLI, Azure PowerShell, azd),
  AZURE_CLIENT_ID, AZURE_TENANT_ID, AZURE_AI_ACCESS_TOKEN

EXIT CODES
  0  success
  1  usage, configuration or input error
  2  translation failed: the source was written unchanged (the structure check failed, or no segment could be
     translated, e.g. the Azure service was unreachable or rejected the credentials); -analyzeOnly failed
  3  partially translated: some segments were kept in the source language (details on stderr and in -reportFile)
  Errors are always printed on stderr, also with -quiet.
";

/// Exit code when a translation (or analysis) failed.
const EXIT_FAILED: u8 = 2;
/// Exit code when some segments were kept in the source language.
const EXIT_PARTIAL: u8 = 3;

const BOOL_FLAGS: &[&str] = &[
    "review",
    "structuralcontext",
    "nmtfallback",
    "preserveanchors",
    "docstrings",
    "codecomments",
    "frontmatter",
    "mathsingledollar",
    "mdx",
    "reviewtm",
    "reportsegments",
    "dumpextraction",
    "dumpgolden",
    "analyzeonly",
    "listlanguages",
    "quiet",
    "help",
    "version",
];

const VALUE_FLAGS: &[&str] = &[
    "sourcefile",
    "targetfile",
    "lang",
    "filename",
    "reportfile",
    "engine",
    "sourcelanguage",
    "formality",
    "donottranslate",
    "translatedeployment",
    "reviewdeployment",
    "analysisdeployment",
    "analysisfile",
    "tmfile",
    "cachedir",
    "maxconcurrency",
    "languagesfile",
    "glossaryfile",
    "envfile",
];

fn alias(name: &str) -> &str {
    match name {
        "s" | "source" | "input" | "i" => "sourcefile",
        "t" | "target" | "output" | "o" => "targetfile",
        "l" | "language" | "languages" | "targetlanguage" | "to" => "lang",
        "h" | "?" => "help",
        "v" => "version",
        other => other,
    }
}

#[derive(Default)]
struct Args {
    values: HashMap<String, String>,
    flags: HashMap<String, bool>,
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Args, String> {
        let mut a = Args::default();
        let mut it = raw.into_iter().peekable();
        while let Some(arg) = it.next() {
            if arg == "-" || !arg.starts_with('-') {
                return Err(format!("unexpected argument \"{arg}\" (options start with -, e.g. -sourceFile {arg})"));
            }
            let body = arg.trim_start_matches('-');
            let (name, inline) = match body.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (body, None),
            };
            let norm: String = name.chars().filter(|c| *c != '-' && *c != '_').collect::<String>().to_lowercase();
            let norm = alias(&norm).to_string();
            if VALUE_FLAGS.contains(&norm.as_str()) {
                let v = match inline {
                    Some(v) => v,
                    None => it.next().ok_or_else(|| format!("{arg} needs a value"))?,
                };
                a.values.insert(norm, v);
            } else if BOOL_FLAGS.contains(&norm.as_str()) {
                let v = match inline {
                    Some(v) => parse_bool(&v),
                    None => match it.peek().map(|n| n.to_ascii_lowercase()) {
                        Some(n) if ["true", "false", "on", "off", "yes", "no", "1", "0"].contains(&n.as_str()) => parse_bool(&it.next().unwrap()),
                        _ => true,
                    },
                };
                a.flags.insert(norm, v);
            } else if let Some(neg) = norm.strip_prefix("no").filter(|n| BOOL_FLAGS.contains(n)) {
                a.flags.insert(neg.to_string(), false);
            } else {
                return Err(format!("unknown option {arg}"));
            }
        }
        Ok(a)
    }

    fn value(&self, k: &str) -> Option<&str> {
        self.values.get(k).map(String::as_str)
    }

    fn flag(&self, k: &str) -> Option<bool> {
        self.flags.get(k).copied()
    }

    fn on(&self, k: &str) -> bool {
        self.flag(k).unwrap_or(false)
    }
}

/// Variables from a .env file (KEY=VALUE, optional quotes, # comments).
fn read_env_file(path: &str) -> Result<HashMap<String, String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim();
        let v = if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2) || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2) {
            v[1..v.len() - 1].to_string()
        } else {
            v.split(" #").next().unwrap_or(v).trim().to_string()
        };
        out.insert(k.trim().to_string(), v);
    }
    Ok(out)
}

/// Loads a .env file into the process environment, like `process.loadEnvFile` in the TypeScript script, so that the
/// configuration and the credential chain (service principal, workload identity, authority host) both see it.
/// Variables that the environment sets win. Called before the async runtime starts any thread.
fn load_env_file(path: &str) -> Result<(), String> {
    for (k, v) in read_env_file(path)? {
        let settable = !k.is_empty() && !k.contains(['=', '\0']) && !v.contains('\0');
        if settable && std::env::var_os(&k).is_none_or(|cur| cur.to_string_lossy().trim().is_empty()) {
            std::env::set_var(&k, &v);
        }
    }
    Ok(())
}

fn read_input(path: &str) -> Result<String, String> {
    let bytes = if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).map_err(|e| format!("stdin: {e}"))?;
        buf
    } else {
        std::fs::read(path).map_err(|e| format!("{path}: {e}"))?
    };
    String::from_utf8(bytes).map_err(|_| format!("{path}: not valid UTF-8"))
}

fn write_output(path: &str, text: &str) -> Result<(), String> {
    if path == "-" {
        let mut out = std::io::stdout().lock();
        out.write_all(text.as_bytes()).and_then(|_| out.flush()).map_err(|e| format!("stdout: {e}"))
    } else {
        if let Some(dir) = std::path::Path::new(path).parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))
    }
}

fn read_json(path: &str) -> Result<Value, String> {
    serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?).map_err(|e| format!("{path}: {e}"))
}

/// `{"tm": [[id, text]...] | {id: text}, "outcomes": [...]}` (the format of `-reportFile` language entries).
fn read_memory(path: &str, review: bool) -> Result<Memory, String> {
    let v = read_json(path)?;
    let root = v.get("languages").and_then(|l| l.get(0)).unwrap_or(&v);
    let tm_value = root.get("tm").ok_or_else(|| format!("{path}: missing \"tm\""))?;
    let mut tm = TMap::new();
    match tm_value {
        Value::Array(entries) => {
            for e in entries {
                if let (Some(id), Some(text)) = (e.get(0).and_then(|x| x.as_str()), e.get(1).and_then(|x| x.as_str())) {
                    tm.insert(id.to_string(), text.to_string());
                }
            }
        }
        Value::Object(m) => {
            for (k, v) in m {
                if let Some(t) = v.as_str() {
                    tm.insert(k.clone(), t.to_string());
                }
            }
        }
        _ => return Err(format!("{path}: \"tm\" must be an array of [id, text] or an object")),
    }
    let outcomes: Vec<SegmentOutcome> = root
        .get("outcomes")
        .and_then(|o| o.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|o| {
                    Some(SegmentOutcome {
                        id: o.get("id")?.as_str()?.to_string(),
                        kind: o.get("kind").and_then(|k| k.as_str()).unwrap_or("").to_string(),
                        via: o.get("via").and_then(|k| k.as_str()).unwrap_or("tm").to_string(),
                        retries: o.get("retries").and_then(|r| r.as_u64()).unwrap_or(0) as u32,
                        errors: o.get("errors").and_then(|e| serde_json::from_value(e.clone()).ok()),
                        review: None,
                        untranslated: o.get("untranslated").and_then(|u| u.as_bool()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Memory::Saved { tm, outcomes, review })
}

fn log(quiet: bool, msg: &str) {
    if !quiet {
        eprintln!("{msg}");
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("mdtranslate: {e}");
            ExitCode::from(1)
        }
    }
}

/// The -targetFile value. Output never goes to stdout unless it is explicitly `-`.
fn target_file(args: &Args) -> Result<String, String> {
    args.value("targetfile").map(String::from).ok_or_else(|| "-targetFile is required (use -targetFile - to write to stdout)".to_string())
}

fn run() -> Result<ExitCode, String> {
    let args = Args::parse(std::env::args().skip(1).collect())?;
    if args.on("help") {
        print!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    }
    if args.on("version") {
        eprintln!("mdtranslate {} (prompts {})", env!("CARGO_PKG_VERSION"), mdtranslate::translate::prompts::PROMPT_VERSION);
        return Ok(ExitCode::SUCCESS);
    }
    if args.value("reportfile") == Some("-") {
        return Err("-reportFile needs a file name; stdout only carries the output of -targetFile -".into());
    }
    let quiet = args.on("quiet");
    if let Some(p) = args.value("envfile") {
        load_env_file(p)?;
    }
    let lookup = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let mut cfg = AppConfig::from_lookup(&lookup);
    if let Some(v) = args.value("cachedir") {
        cfg.cache_dir = Some(v.to_string());
    }
    if let Some(v) = args.value("maxconcurrency") {
        cfg.max_concurrency = v.parse().map_err(|_| format!("-maxConcurrency: not a number: {v}"))?;
    }
    if let Some(v) = args.value("analysisdeployment") {
        cfg.analysis_deployment = v.to_string();
    }
    let catalog = load_languages(args.value("languagesfile").or(cfg.languages_file.as_deref()))?;
    let glossary = load_glossary(args.value("glossaryfile").or(cfg.glossary_file.as_deref()))?;
    if args.on("listlanguages") {
        let target = target_file(&args)?;
        let langs: Vec<Value> = catalog.order.iter().map(|c| serde_json::to_value(&catalog.languages[c]).unwrap()).collect();
        write_output(&target, &serde_json::to_string_pretty(&json!({ "defaultTargets": catalog.default_targets, "languages": langs })).unwrap())?;
        return Ok(ExitCode::SUCCESS);
    }

    let source = args.value("sourcefile").map(String::from).ok_or("-sourceFile is required (use -sourceFile - to read stdin)")?;
    let target = target_file(&args)?;
    let file_name = args
        .value("filename")
        .map(String::from)
        .unwrap_or_else(|| if source == "-" { "stdin.md".into() } else { std::path::Path::new(&source).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(source.clone()) });
    let content = read_input(&source)?;

    let opts = TranslateOptions {
        review: args.flag("review"),
        structural_context: args.flag("structuralcontext"),
        nmt_fallback: args.flag("nmtfallback"),
        preserve_anchors: args.flag("preserveanchors"),
        docstrings: args.flag("docstrings"),
        code_comments: args.flag("codecomments"),
        front_matter: args.flag("frontmatter"),
        math_single_dollar: args.flag("mathsingledollar"),
        mdx: args.flag("mdx"),
        formality: match args.value("formality") {
            Some(f) => Some(parse_formality(f).ok_or_else(|| format!("-formality must be formal or informal, not \"{f}\""))?),
            None => None,
        },
        source_language: args.value("sourcelanguage").map(String::from),
        do_not_translate: args.value("donottranslate").map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default(),
        engine: match args.value("engine") {
            Some(e) => Some(Engine::parse(e).ok_or_else(|| format!("-engine must be gpt, nmt or pseudo, not \"{e}\""))?),
            None => None,
        },
        translate_deployment: args.value("translatedeployment").map(String::from),
        review_deployment: args.value("reviewdeployment").map(String::from),
    };
    let translator = MarkdownTranslator::new(cfg, catalog, glossary)?;

    if args.on("dumpgolden") {
        let eo = translator.extract_options(&file_name, &opts);
        let v = Variant {
            mdx: eo.parse.mdx,
            math_single_dollar: eo.parse.math_single_dollar,
            docstrings: eo.docstrings,
            code_comments: eo.code_comments,
            front_matter: eo.front_matter,
            preserve_anchors: opts.preserve_anchors.unwrap_or(translator.cfg.preserve_anchors),
        };
        write_output(&target, &serde_json::to_string(&golden(&content, &translator.do_not_translate(&opts.do_not_translate), &v)).unwrap())?;
        return Ok(ExitCode::SUCCESS);
    }
    if args.on("dumpextraction") {
        let ex = translator.extract(&file_name, &content, &opts)?;
        write_output(&target, &serde_json::to_string(&Value::Object(dump_extraction(&ex))).unwrap())?;
        return Ok(ExitCode::SUCCESS);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    runtime.block_on(translate(&args, &translator, &file_name, &content, &target, &opts, quiet))
}

async fn translate(args: &Args, translator: &MarkdownTranslator, file_name: &str, content: &str, target: &str, opts: &TranslateOptions, quiet: bool) -> Result<ExitCode, String> {
    let started = Instant::now();
    let analysis: Option<Option<DocAnalysis>> = match args.value("analysisfile") {
        Some(p) => {
            let v = read_json(p)?;
            let a = v.get("analysis").cloned().unwrap_or(v);
            Some(if a.is_null() { None } else { Some(serde_json::from_value(a).map_err(|e| format!("{p}: {e}"))?) })
        }
        None => None,
    };
    if args.on("analyzeonly") {
        let a = match analysis {
            Some(a) => a,
            None => {
                if !translator.chat.available() {
                    return Err("-analyzeOnly needs a model: set AZURE_OPENAI_ENDPOINT".into());
                }
                let ex = translator.extract(file_name, content, opts)?;
                if ex.active().next().is_some() {
                    match translator.analyze(file_name, &ex.source).await {
                        Ok(a) => a,
                        Err(e) => {
                            eprintln!("mdtranslate: {file_name}: {e}");
                            return Ok(ExitCode::from(EXIT_FAILED));
                        }
                    }
                } else {
                    None
                }
            }
        };
        let out = json!({ "file": file_name, "analysis": a, "usage": &*translator.chat.usage.lock().unwrap() });
        write_output(target, &serde_json::to_string_pretty(&out).unwrap())?;
        return Ok(ExitCode::SUCCESS);
    }

    let codes: Vec<String> = args.value("lang").map(|l| l.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
    if codes.is_empty() {
        return Err("-lang is required (for example -lang de or -lang de,fr); -listLanguages shows the catalog".into());
    }
    let targets = translator.resolve_languages(&codes).map_err(|u| format!("unknown target language \"{}\" (see -listLanguages)", u.0))?;
    if targets.len() > 1 && !target.contains("{lang}") {
        return Err("with several -lang values the -targetFile name must contain {lang}".into());
    }
    let memory = match args.value("tmfile") {
        Some(p) => read_memory(p, args.on("reviewtm"))?,
        None => Memory::Engine,
    };
    if matches!(memory, Memory::Engine) && translator.engine(opts) != Engine::Pseudo {
        let ready = match translator.engine(opts) {
            Engine::Gpt => translator.chat.available() || translator.nmt.available(),
            Engine::Nmt => translator.nmt.available(),
            Engine::Pseudo => true,
        };
        if !ready {
            return Err("no translation service configured: set AZURE_OPENAI_ENDPOINT (and/or AZURE_TRANSLATOR_ENDPOINT), or use -engine pseudo".into());
        }
    }

    let result = translator.translate_file(file_name, content, &targets, opts, analysis, &memory).await?;
    if let Some(e) = &result.analysis_error {
        log(quiet, &format!("mdtranslate: warning: {file_name}: {e}; translating without document analysis"));
    }
    let mut worst = Status::Translated;
    let mut languages = Vec::new();
    for (lang, r) in targets.iter().zip(result.languages.iter()) {
        let path = target.replace("{lang}", &lang.code);
        write_output(&path, &r.text)?;
        let rep = &r.report;
        let status = r.status();
        worst = worst.max(status);
        // Errors are reported on stderr even with -quiet.
        let reason = rep.error.clone().or_else(|| r.first_segment_error().map(String::from)).unwrap_or_else(|| "validation failed".into());
        match status {
            Status::Failed if rep.error.is_some() => eprintln!("mdtranslate: {file_name} -> {}: {reason} (source written unchanged)", lang.code),
            Status::Failed => eprintln!("mdtranslate: {file_name} -> {}: no segment could be translated: {reason} (source written unchanged)", lang.code),
            Status::Partial => eprintln!("mdtranslate: {file_name} -> {}: {} of {} segments kept in the source language: {reason}", lang.code, rep.kept_source.len(), rep.segments),
            Status::Translated => {}
        }
        log(
            quiet,
            &format!(
                "{file_name} -> {}: {} segments ({}), {} reverted for structure, {} kept in source language{}",
                lang.code,
                rep.segments,
                rep.via.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "),
                rep.reverted_for_structure.len(),
                rep.kept_source.len(),
                if path == "-" { String::new() } else { format!(" -> {path}") }
            ),
        );
        let mut entry = serde_json::to_value(rep).unwrap();
        entry["status"] = Value::String(status.as_str().into());
        entry["outputFile"] = Value::String(path.clone());
        let mut tm: Vec<(&String, &String)> = r.tm.iter().collect();
        let order: HashMap<&str, usize> = result.extraction.segments().iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
        tm.sort_by_key(|(id, _)| order.get(id.as_str()).copied().unwrap_or(usize::MAX));
        entry["tm"] = json!(tm.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>());
        entry["outcomes"] = serde_json::to_value(r.outcomes.values()).unwrap();
        if args.on("reportsegments") {
            let segs: Vec<Value> = result
                .extraction
                .active()
                .map(|s| {
                    json!({
                        "id": s.id, "kind": s.kind.as_str(), "note": s.note, "masked": s.masked, "original": s.original,
                        "rendered": render_segment(s, &r.tm, &result.extraction.store, RenderOptions::wrapped(false)),
                    })
                })
                .collect();
            entry["segmentsDetail"] = Value::Array(segs);
        }
        languages.push(entry);
    }
    if let Some(p) = args.value("reportfile") {
        let usage = translator.chat.usage.lock().unwrap().clone();
        let report = json!({
            "file": file_name,
            "engine": translator.engine(opts).as_str(),
            "seconds": started.elapsed().as_secs_f64(),
            "sourceLanguage": result.source_language,
            "formality": result.formality.as_str(),
            "analysis": result.analysis,
            "analysisError": result.analysis_error,
            "notes": result.extraction.notes,
            "languages": languages,
            "usage": usage,
        });
        write_output(p, &serde_json::to_string_pretty(&report).unwrap())?;
    }
    Ok(ExitCode::from(match worst {
        Status::Translated => 0,
        Status::Failed => EXIT_FAILED,
        Status::Partial => EXIT_PARTIAL,
    }))
}
