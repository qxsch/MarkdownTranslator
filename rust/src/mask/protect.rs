//! Detection of spans that must never be translated inside running text: URLs, file names, paths,
//! identifiers, variables, CLI flags, versions, etc. Port of `protect.ts`.

use crate::markdown::math_spans::find_dollar_math;
use crate::regexutil::{escape_literal, jsf, RegexExt};
use fancy_regex::Regex;
use std::sync::OnceLock;

const FILE_EXTENSIONS: &[&str] = &[
    "md", "mdx", "markdown", "txt", "rst", "adoc", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "csv", "tsv", "rtf", "odt",
    "png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico", "tif", "tiff", "avif", "heic", "psd", "excalidraw", "drawio", "vsdx",
    "mp3", "mp4", "wav", "mov", "avi", "mkv", "webm", "ogg", "flac",
    "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "nupkg", "whl", "jar", "war", "ear", "deb", "rpm", "msi", "exe", "dll", "so", "dylib", "bin", "iso", "vhd", "vhdx", "img", "dmg", "apk", "appx", "msix",
    "js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx", "vue", "svelte", "astro", "json", "jsonc", "json5", "jsonl", "ndjson", "yaml", "yml", "toml", "ini", "cfg", "conf", "config", "env", "properties", "xml", "xsd", "xsl", "xslt", "plist",
    "html", "htm", "xhtml", "css", "scss", "sass", "less", "styl",
    "py", "pyi", "pyc", "ipynb", "r", "rmd", "jl", "rb", "erb", "php", "pl", "pm", "lua", "go", "rs", "java", "kt", "kts", "scala", "groovy", "gradle", "swift", "m", "mm", "c", "h", "cc", "cpp", "cxx", "hpp", "hh", "cs", "csx", "fs", "fsx", "vb", "dart", "ex", "exs", "erl", "hs", "clj", "elm", "zig", "nim", "sol",
    "sh", "bash", "zsh", "fish", "ps1", "psm1", "psd1", "ps1xml", "bat", "cmd", "vbs", "awk", "sed",
    "sql", "db", "sqlite", "bak", "mdf", "ldf", "parquet", "avro", "orc", "onnx", "pt", "pth", "pkl", "h5", "safetensors", "gguf",
    "bicep", "bicepparam", "tf", "tfvars", "tfstate", "hcl", "nomad", "arm", "template", "dockerfile", "containerfile", "helmignore", "tpl",
    "csproj", "vbproj", "fsproj", "sln", "slnx", "props", "targets", "nuspec", "vcxproj", "pbxproj", "xcodeproj", "resx", "razor", "cshtml", "xaml", "axaml",
    "lock", "log", "pem", "crt", "cer", "key", "pfx", "p12", "jks", "pub", "gpg", "asc", "sig", "rules", "service", "socket", "timer", "desktop",
    "woff", "woff2", "ttf", "otf", "eot", "map", "wasm", "proto", "graphql", "gql", "prisma", "http", "rest", "har", "pbix", "pbit", "kql", "csl",
];

const DOTFILES: &[&str] = &[
    "env", "gitignore", "gitattributes", "gitmodules", "gitkeep", "editorconfig", "npmrc", "nvmrc", "yarnrc", "dockerignore", "prettierrc", "prettierignore",
    "eslintrc", "eslintignore", "babelrc", "browserslistrc", "stylelintrc", "vscode", "github", "git", "bashrc", "zshrc", "profile", "bash_profile", "devcontainer",
    "azure", "terraform", "terraformrc", "kube", "ssh", "aws", "config", "pylintrc", "flake8", "coveragerc", "markdownlint", "vsconfig", "funcignore", "venv",
];

const SPECIAL_FILES: &[&str] = &[
    "Dockerfile", "Containerfile", "Makefile", "Jenkinsfile", "Vagrantfile", "Procfile", "Gemfile", "Rakefile", "Brewfile", "Pipfile", "Justfile", "Taskfile",
    "CODEOWNERS", "LICENSE", "README", "CHANGELOG", "CONTRIBUTING", "SECURITY", "NOTICE", "AUTHORS", "OWNERS", "MAINTAINERS", "go.mod", "go.sum", "Cargo.toml",
];

const PS_VERBS: &[&str] = &[
    "Add", "Approve", "Assert", "Backup", "Block", "Build", "Checkpoint", "Clear", "Close", "Compare", "Complete", "Compress", "Confirm", "Connect", "Convert",
    "ConvertFrom", "ConvertTo", "Copy", "Debug", "Deny", "Deploy", "Disable", "Disconnect", "Dismount", "Edit", "Enable", "Enter", "Exit", "Expand", "Export",
    "Find", "ForEach", "Format", "Get", "Grant", "Group", "Hide", "Import", "Initialize", "Install", "Invoke", "Join", "Limit", "Lock", "Measure", "Merge",
    "Mount", "Move", "New", "Open", "Optimize", "Out", "Ping", "Pop", "Protect", "Publish", "Push", "Read", "Receive", "Redo", "Register", "Remove", "Rename",
    "Repair", "Request", "Reset", "Resize", "Resolve", "Restart", "Restore", "Resume", "Revoke", "Save", "Search", "Select", "Send", "Set", "Show", "Skip",
    "Sort", "Split", "Start", "Step", "Stop", "Submit", "Suspend", "Switch", "Sync", "Tee", "Test", "Trace", "Unblock", "Undo", "Uninstall", "Unlock",
    "Unprotect", "Unpublish", "Unregister", "Update", "Use", "Wait", "Watch", "Where", "Write",
];

// Path/identifier "word" characters; markdown escapes like "\_" are allowed inside identifiers.
const W: &str = r"(?:[\w$@+~-]|\\[_*])";
const BOUNDARY_END: &str = r#"(?=$|[\s"'`)\]}>,;:!?]|\.(?:$|\s))"#;

pub struct Rule {
    pub name: &'static str,
    pub re: Regex,
}

fn join(list: &[&str]) -> String {
    list.iter().map(|s| escape_literal(s)).collect::<Vec<_>>().join("|")
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let r = |name: &'static str, pattern: &str, flags: &str| Rule { name, re: jsf(pattern, flags) };
        vec![
            // GitHub / Microsoft Docs alerts and directives: > [!NOTE], > [!div class="..."]
            r("alert", r"\[![A-Za-z]+(?:\s[^\]\n]*)?\]", ""),
            // Microsoft Docs includes and embeds: [!INCLUDE [title](path)], [!VIDEO url]
            r("docsinclude", r"\[!(?:INCLUDE|VIDEO|div|code)\b\s*", "i"),
            r("url", r#"\b(?:https?|ftp|ftps|sftp|file|ssh|git|s3|wss?|abfss?|wasbs?|vscode|mailto|tel|data|urn):(?:\/\/)?[^\s<>"'`]+[^\s<>"'`.,;:!?)\]}]"#, "i"),
            r("www", r#"\bwww\.[\w-]+(?:\.[\w-]+)+(?:\/[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}])?"#, "i"),
            r("email", r"\b[\w.+-]+@[\w-]+(?:\.[\w-]+)+\b", ""),
            r("uncpath", r#"\\\\[\w.$-]+(?:\\[^\s\\<>"'`|*?]+)+\\?"#, ""),
            r("winpath", r#"\b[A-Za-z]:\\(?:[^\s\\<>"'`|*?]+\\?)*"#, ""),
            r("envpath", r#"(?:%[A-Za-z_][\w]*%|\$env:[A-Za-z_]\w*|\$\{?[A-Za-z_]\w*\}?)(?:[\\/][^\s<>"'`|*?]+)+"#, ""),
            r("abspath", &format!(r"(?<![\w/\\.])(?:~|\.{{1,2}})?/{W}+(?:[./]{W}+)*/?{BOUNDARY_END}"), ""),
            // "and/or" style word pairs are prose; a relative path needs an extension or at least two separators.
            r("relpath", &format!(r"(?<![\w/\\]){W}+(?:\.{W}+)*(?:/{W}+(?:\.{W}+)*)*/{W}+\.[A-Za-z0-9]{{1,10}}{BOUNDARY_END}"), ""),
            r("deeppath", &format!(r"(?<![\w/\\]){W}+(?:\.{W}+)*(?:/{W}+(?:\.{W}+)*){{2,}}/?{BOUNDARY_END}"), ""),
            r("dirpath", &format!(r"(?<![\w/\\]){W}+(?:\.{W}+)*/{BOUNDARY_END}"), ""),
            r("filename", &format!(r"(?<![\w/\\.-]){W}+(?:\.{W}+)*\.(?:{})(?![\w-])", join(FILE_EXTENSIONS)), "i"),
            r("dotfile", &format!(r"(?<![\w.])\.(?:{})(?:\.[\w-]+)*(?![\w-])", join(DOTFILES)), ""),
            r("specialfile", &format!(r"\b(?:{})(?![\w-])", join(SPECIAL_FILES)), ""),
            r("template", r"\{\{[^{}\n]{1,120}\}\}|\$\{[^{}\n]{1,120}\}|\{\{?[A-Za-z_][\w.-]*\}\}?|\{\d+(?::[^{}]*)?\}|<%[=-]?[^%]{1,120}%>|\[\[[^\]\n]{1,80}\]\]", ""),
            r("envvar", r"\$env:[A-Za-z_][\w]*|%[A-Za-z_][\w]*%|\$[A-Za-z_][\w]*|\$\{[A-Za-z_]\w*\}", ""),
            r("printf", r"(?<![\w%])%(?:\d+\$)?[-+0#]*\d*(?:\.\d+)?[sdifuxXoecg](?![\w])", ""),
            r("emoji", r"(?<![\w:]):[a-z0-9_+-]{2,40}:(?![\w:])", ""),
            r("longflag", r#"(?<![\w-])--[A-Za-z0-9][\w-]*(?:=[^\s<>"'`]+)?"#, ""),
            // JavaScript: (?<=^|[\s([]) — rewritten without a variable-length lookbehind.
            r("shortflag", r"(?:^|(?<=[\s([]))-[A-Za-z]{1,2}(?=$|[\s,.;:)\]])", ""),
            r("guid", r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b", ""),
            r("version", r"\bv\d+(?:\.\d+)+(?:[-+][\w.-]+)?\b|\b\d+\.\d+\.\d+(?:\.\d+)?(?:[-+][\w.-]+)?\b", ""),
            r("ip", r"\b(?:\d{1,3}\.){3}\d{1,3}(?:\/\d{1,2})?(?::\d{1,5})?\b", ""),
            r("hostport", r"\b(?:localhost|127\.0\.0\.1|0\.0\.0\.0)(?::\d{1,5})?\b", ""),
            r("hex", r"(?<![\w#])#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b|\b0x[0-9a-fA-F]+\b", ""),
            r("issue", r"(?<![\w&#])#\d+\b", ""),
            r("mention", r"(?<![\w@.])@[A-Za-z0-9][\w-]*(?:\/[\w.-]+)?", ""),
            r("cmdlet", &format!(r"\b(?:{})-[A-Z][A-Za-z0-9]+\b", PS_VERBS.join("|")), ""),
            r("call", r"\b[A-Za-z_$][\w$]*(?:(?:\.|::|->)[A-Za-z_$][\w$]*)*\(\)", ""),
            r("qualified", r"\b[A-Za-z_][\w]*(?:(?:::|->)[A-Za-z_][\w]*)+\b|\b[A-Z][\w]*(?:\.[A-Z][\w]*){1,}\b|\b[a-z_][\w]*(?:\.[a-z_][\w]*)+\b(?=\(|\s*=)", ""),
            r("snake", r"\b[A-Za-z][A-Za-z0-9]*(?:\\?_[A-Za-z0-9]+)+\b", ""),
            r("camel", r"\b[a-z]+[0-9]*(?:[A-Z][a-z0-9]*)+\b", ""),
            r("pascal", r"\b[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]*)+\b|\b[A-Z]{2,}[a-z]+[A-Za-z0-9]*\b", ""),
        ]
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub rule: &'static str,
}

fn dnt_regex(term: &str) -> Regex {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<String, Regex>> = Default::default();
    }
    CACHE.with(|c| {
        c.borrow_mut()
            .entry(term.to_string())
            .or_insert_with(|| {
                let pattern = format!(r"(?<![\p{{L}}\p{{N}}_]){}(?![\p{{L}}\p{{N}}_])", regex::escape(term));
                Regex::new(&pattern).expect("escaped do-not-translate term")
            })
            .clone()
    })
}

pub fn find_protected(text: &str, do_not_translate: &[String]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut collect = |re: &Regex, rule: &'static str| {
        for m in re.all(text) {
            if m.is_empty() {
                continue;
            }
            spans.push(Span { start: m.start, end: m.end, rule });
        }
    };
    for r in rules() {
        collect(&r.re, r.name);
    }
    // Formulas such as $\alpha = 0.7$ stay verbatim even when single-dollar math is not parsed.
    for (start, end) in find_dollar_math(text) {
        spans.push(Span { start, end, rule: "math" });
    }
    for term in do_not_translate {
        if crate::jsstr::trim(term).is_empty() {
            continue;
        }
        let re = dnt_regex(term);
        for m in re.all(text) {
            if !m.is_empty() {
                spans.push(Span { start: m.start, end: m.end, rule: "dnt" });
            }
        }
    }
    merge_spans(spans)
}

/// Keeps non-overlapping spans; the earliest start wins, then the longest.
pub fn merge_spans(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut out: Vec<Span> = Vec::new();
    for s in spans {
        if let Some(last) = out.last() {
            if s.start < last.end {
                continue;
            }
        }
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protected(text: &str) -> Vec<(&str, &'static str)> {
        find_protected(text, &[]).into_iter().map(|s| (&text[s.start..s.end], s.rule)).collect()
    }

    #[test]
    fn rules_compile_and_match() {
        let p = protected("See test.jpeg, ./scripts/deploy.ps1 and https://example.com/x. Run --port 8080 -v on v1.2.3.");
        assert!(p.contains(&("test.jpeg", "filename")), "{p:?}");
        assert!(p.contains(&("./scripts/deploy.ps1", "abspath")), "{p:?}");
        assert!(p.contains(&("https://example.com/x", "url")), "{p:?}");
        assert!(p.contains(&("--port", "longflag")), "{p:?}");
        assert!(p.contains(&("-v", "shortflag")), "{p:?}");
        assert!(p.contains(&("v1.2.3", "version")), "{p:?}");
    }

    #[test]
    fn identifiers() {
        let p = protected("Set MDT_REVIEW and myValue in MyClass.Method() via Get-AzResource.");
        assert!(p.contains(&("MDT_REVIEW", "snake")), "{p:?}");
        assert!(p.contains(&("myValue", "camel")), "{p:?}");
        assert!(p.contains(&("MyClass.Method()", "call")), "{p:?}");
        assert!(p.contains(&("Get-AzResource", "cmdlet")), "{p:?}");
    }

    #[test]
    fn prose_is_not_protected() {
        assert!(protected("This guide explains how to deploy the service and/or configure it.").is_empty());
    }

    #[test]
    fn do_not_translate_terms() {
        let dnt = vec!["Contoso Sync".to_string(), "C++".to_string()];
        let s = find_protected("Install Contoso Sync and C++ tools.", &dnt);
        let texts: Vec<_> = s.iter().map(|s| (&"Install Contoso Sync and C++ tools."[s.start..s.end], s.rule)).collect();
        assert!(texts.contains(&("Contoso Sync", "dnt")), "{texts:?}");
        assert!(texts.contains(&("C++", "dnt")), "{texts:?}");
    }
}
