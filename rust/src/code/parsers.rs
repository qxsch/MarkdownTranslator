//! Comment and docstring detection in fenced code blocks: tree-sitter grammars compiled into the binary
//! for 15 languages, a small lexer for many more. Port of `code/parsers.ts`.

use crate::jsstr;
use crate::regexutil::RegexExt;
use crate::rx;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommentRange {
    pub start: usize,
    pub end: usize,
}

fn grammar_for(key: &str) -> Option<&'static str> {
    Some(match key {
        "javascript" | "js" | "jsx" | "mjs" | "cjs" | "node" | "nodejs" => "javascript",
        "typescript" | "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "python" | "py" | "python3" | "py3" | "gyp" => "python",
        "bash" | "sh" | "shell" | "zsh" | "ksh" | "shellscript" => "bash",
        "powershell" | "ps1" | "pwsh" | "ps" | "posh" | "psm1" => "powershell",
        "csharp" | "cs" | "c#" => "c-sharp",
        "cpp" | "c++" | "cxx" | "cc" | "hpp" | "c" | "h" | "arduino" | "cuda" => "cpp",
        "css" => "css",
        "go" | "golang" => "go",
        "java" => "java",
        "php" => "php",
        "ruby" | "rb" => "ruby",
        "rust" | "rs" => "rust",
        "ini" | "cfg" | "dosini" | "properties" | "editorconfig" | "gitconfig" => "ini",
        _ => return None,
    })
}

/// `lang.trim().toLowerCase().replace(/^\{?\.?/, '').split(/[\s{,]/)[0]`
pub fn lang_key(lang: &str) -> String {
    let l = jsstr::trim(lang).to_lowercase();
    let l = l.strip_prefix('{').unwrap_or(&l);
    let l = l.strip_prefix('.').unwrap_or(l);
    l.split(|c: char| jsstr::is_js_space(c) || c == '{' || c == ',').next().unwrap_or("").to_string()
}

// ------------------------------------------------------------------ tree-sitter

#[cfg(feature = "grammars")]
mod ts {
    use super::CommentRange;
    use crate::regexutil::RegexExt;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use tree_sitter::{Language, Node, Parser, Tree};

    const COMMENT_TYPES: &[&str] = &["comment", "line_comment", "block_comment", "doc_comment"];

    fn language(grammar: &str) -> Language {
        match grammar {
            "javascript" => tree_sitter_javascript::LANGUAGE.into(),
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
            "python" => tree_sitter_python::LANGUAGE.into(),
            "bash" => tree_sitter_bash::LANGUAGE.into(),
            "powershell" => tree_sitter_powershell::LANGUAGE.into(),
            "c-sharp" => tree_sitter_c_sharp::LANGUAGE.into(),
            "cpp" => tree_sitter_cpp::LANGUAGE.into(),
            "css" => tree_sitter_css::LANGUAGE.into(),
            "go" => tree_sitter_go::LANGUAGE.into(),
            "java" => tree_sitter_java::LANGUAGE.into(),
            "php" => tree_sitter_php::LANGUAGE_PHP.into(),
            "ruby" => tree_sitter_ruby::LANGUAGE.into(),
            "rust" => tree_sitter_rust::LANGUAGE.into(),
            "ini" => tree_sitter_ini::LANGUAGE.into(),
            other => unreachable!("unknown grammar {other}"),
        }
    }

    thread_local! {
        static PARSERS: RefCell<HashMap<&'static str, Parser>> = RefCell::new(HashMap::new());
    }

    pub fn parse(grammar: &'static str, code: &str) -> Option<Tree> {
        PARSERS.with(|p| {
            let mut map = p.borrow_mut();
            let parser = map.entry(grammar).or_insert_with(|| {
                let mut parser = Parser::new();
                parser.set_language(&language(grammar)).expect("grammar ABI is supported");
                parser
            });
            parser.reset();
            parser.parse(code, None)
        })
    }

    /// All descendants (named or anonymous) in document order.
    fn descendants<'t>(root: Node<'t>, out: &mut Vec<Node<'t>>) {
        let mut cursor = root.walk();
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            out.push(n);
            let children: Vec<Node<'t>> = n.children(&mut cursor).collect();
            for c in children.into_iter().rev() {
                stack.push(c);
            }
        }
    }

    pub fn comments(grammar: &'static str, code: &str) -> Vec<CommentRange> {
        let Some(tree) = parse(grammar, code) else {
            return Vec::new();
        };
        let mut all = Vec::new();
        descendants(tree.root_node(), &mut all);
        let bytes = code.as_bytes();
        let mut out: Vec<CommentRange> = all
            .into_iter()
            .skip(1)
            .filter(|n| COMMENT_TYPES.contains(&n.kind()) && !n.parent().is_some_and(|p| COMMENT_TYPES.contains(&p.kind())))
            .map(|n| {
                // Some grammars (Rust `///` doc comments) include the line ending in the comment node.
                let mut end = n.end_byte();
                while end > n.start_byte() && matches!(bytes[end - 1], b'\n' | b'\r') {
                    end -= 1;
                }
                CommentRange { start: n.start_byte(), end }
            })
            .collect();
        out.sort_by_key(|c| c.start);
        out
    }

    pub fn parses_as_code(grammar: &'static str, text: &str) -> Option<bool> {
        let tree = parse(grammar, text)?;
        let root = tree.root_node();
        Some(!root.has_error() && root.named_child_count() > 0)
    }

    pub fn docstrings(code: &str) -> Vec<CommentRange> {
        let Some(tree) = parse("python", code) else {
            return Vec::new();
        };
        let root = tree.root_node();
        let mut all = Vec::new();
        descendants(root, &mut all);
        let mut bodies = vec![root];
        for def in all.iter().skip(1).filter(|n| matches!(n.kind(), "function_definition" | "class_definition")) {
            if let Some(body) = def.child_by_field_name("body") {
                bodies.push(body);
            }
        }
        let mut out = Vec::new();
        for body in bodies {
            let mut cursor = body.walk();
            let first = body.named_children(&mut cursor).find(|c| c.kind() != "comment");
            let Some(first) = first else { continue };
            if first.kind() != "expression_statement" {
                continue;
            }
            let Some(s) = first.named_child(0) else { continue };
            if s.kind() != "string" {
                continue;
            }
            if super::TRIPLE_QUOTED().test(&code[s.start_byte()..]) {
                out.push(CommentRange { start: s.start_byte(), end: s.end_byte() });
            }
        }
        out.sort_by_key(|c| c.start);
        out
    }
}

rx!(TRIPLE_QUOTED, r#"^[rRuUbB]{0,2}("""|''')"#);

fn tree_sitter_comments(grammar: &'static str, code: &str) -> Vec<CommentRange> {
    #[cfg(feature = "grammars")]
    {
        ts::comments(grammar, code)
    }
    #[cfg(not(feature = "grammars"))]
    {
        let _ = (grammar, code);
        Vec::new()
    }
}

/// True when the grammar of `lang` parses `text` without errors, false when it rejects it, `None` without a
/// grammar.
pub fn parses_as_code(lang: &str, text: &str) -> Option<bool> {
    let grammar = grammar_for(&lang_key(lang))?;
    #[cfg(feature = "grammars")]
    {
        ts::parses_as_code(grammar, text)
    }
    #[cfg(not(feature = "grammars"))]
    {
        let _ = (grammar, text);
        None
    }
}

/// Python docstrings: the first statement of a module, class or function when it is a triple-quoted string.
pub fn find_docstrings(lang: &str, code: &str) -> Vec<CommentRange> {
    if grammar_for(&lang_key(lang)) != Some("python") {
        return Vec::new();
    }
    #[cfg(feature = "grammars")]
    {
        ts::docstrings(code)
    }
    #[cfg(not(feature = "grammars"))]
    {
        let _ = code;
        Vec::new()
    }
}

// ------------------------------------------------------------------ lexer fallback

#[derive(Clone, Copy)]
struct StrSpec {
    open: &'static str,
    close: &'static str,
    esc: Option<u8>,
    doubled: bool,
}

struct LexSpec {
    line: &'static [&'static str],
    block: &'static [(&'static str, &'static str)],
    strings: &'static [StrSpec],
    line_start_only: bool,
    /// Line comment marker must follow whitespace or line start (YAML, shell-like).
    line_needs_space: bool,
}

const fn s(open: &'static str, close: &'static str) -> StrSpec {
    StrSpec { open, close, esc: None, doubled: false }
}
const fn se(open: &'static str, close: &'static str) -> StrSpec {
    StrSpec { open, close, esc: Some(b'\\'), doubled: false }
}
const fn sd(open: &'static str, close: &'static str) -> StrSpec {
    StrSpec { open, close, esc: None, doubled: true }
}
const DQ: StrSpec = se("\"", "\"");
const SQ: StrSpec = se("'", "'");

const fn lex(line: &'static [&'static str], block: &'static [(&'static str, &'static str)], strings: &'static [StrSpec]) -> LexSpec {
    LexSpec { line, block, strings, line_start_only: false, line_needs_space: false }
}

static BICEP: LexSpec = lex(&["//"], &[("/*", "*/")], &[s("'''", "'''"), SQ]);
static TERRAFORM: LexSpec = lex(&["#", "//"], &[("/*", "*/")], &[DQ]);
static SQL: LexSpec = lex(&["--"], &[("/*", "*/")], &[sd("'", "'"), sd("\"", "\""), s("[", "]")]);
static KUSTO: LexSpec = lex(&["//"], &[], &[DQ, SQ, s("```", "```")]);
static YAML: LexSpec = LexSpec { line_needs_space: true, ..lex(&["#"], &[], &[DQ, sd("'", "'")]) };
static TOML: LexSpec = lex(&["#"], &[], &[se("\"\"\"", "\"\"\""), s("'''", "'''"), DQ, s("'", "'")]);
static DOCKERFILE: LexSpec = LexSpec { line_start_only: true, ..lex(&["#"], &[], &[]) };
static MAKEFILE: LexSpec = LexSpec { line_needs_space: true, ..lex(&["#"], &[], &[]) };
static HASH: LexSpec = LexSpec { line_needs_space: true, ..lex(&["#"], &[], &[DQ, SQ]) };
static XML: LexSpec = lex(&[], &[("<!--", "-->")], &[]);
static LUA: LexSpec = lex(&["--"], &[("--[[", "]]")], &[DQ, SQ, s("[[", "]]")]);
static HASKELL: LexSpec = lex(&["--"], &[("{-", "-}")], &[DQ]);
static VB: LexSpec = lex(&["'"], &[], &[sd("\"", "\"")]);
static BATCH: LexSpec = LexSpec { line_start_only: true, ..lex(&["REM ", "rem ", "::"], &[], &[]) };
static CLIKE: LexSpec = lex(&["//"], &[("/*", "*/")], &[s("\"\"\"", "\"\"\""), DQ, SQ, se("`", "`")]);
static SCSS: LexSpec = lex(&["//"], &[("/*", "*/")], &[DQ, SQ]);
static GRAPHQL: LexSpec = lex(&["#"], &[], &[s("\"\"\"", "\"\"\""), DQ]);
static ERLANG: LexSpec = lex(&["%"], &[], &[DQ]);
static LISP: LexSpec = lex(&[";"], &[], &[DQ]);

fn lex_spec(name: &str) -> Option<&'static LexSpec> {
    Some(match name {
        "bicep" => &BICEP,
        "terraform" => &TERRAFORM,
        "sql" => &SQL,
        "kusto" => &KUSTO,
        "yaml" => &YAML,
        "toml" => &TOML,
        "dockerfile" => &DOCKERFILE,
        "makefile" => &MAKEFILE,
        "hash" => &HASH,
        "xml" => &XML,
        "lua" => &LUA,
        "haskell" => &HASKELL,
        "vb" => &VB,
        "batch" => &BATCH,
        "clike" => &CLIKE,
        "scss" => &SCSS,
        "graphql" => &GRAPHQL,
        "erlang" => &ERLANG,
        "lisp" => &LISP,
        _ => return None,
    })
}

fn lex_alias(key: &str) -> Option<&'static str> {
    Some(match key {
        "bicep" | "bicepparam" => "bicep",
        "hcl" | "terraform" | "tf" | "tfvars" => "terraform",
        "sql" | "tsql" | "t-sql" | "mysql" | "postgresql" | "postgres" | "psql" | "plsql" | "sqlite" | "mssql" | "pgsql" => "sql",
        "kql" | "kusto" | "csl" => "kusto",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "dockerfile" | "docker" | "containerfile" => "dockerfile",
        "makefile" | "make" | "mk" => "makefile",
        "r" | "perl" | "pl" | "elixir" | "ex" | "exs" | "nginx" | "apache" | "conf" | "cmake" | "julia" | "jl" | "nim" | "tcl" | "gitignore" | "dotenv" | "env"
        | "coffee" | "crystal" | "awk" | "fish" | "starlark" | "bazel" | "gdscript" => "hash",
        "xml" | "html" | "xhtml" | "svg" | "xaml" | "csproj" | "msbuild" | "plist" | "vue" | "razor" | "cshtml" => "xml",
        "lua" => "lua",
        "haskell" | "hs" | "elm" => "haskell",
        "vb" | "vbnet" | "vb.net" | "vba" | "vbscript" | "vbs" => "vb",
        "bat" | "batch" | "cmd" => "batch",
        "kotlin" | "kt" | "kts" | "swift" | "scala" | "dart" | "groovy" | "gradle" | "jsonc" | "json5" | "proto" | "protobuf" | "solidity" | "sol" | "zig"
        | "objectivec" | "objc" | "objective-c" | "fsharp" | "fs" | "glsl" | "hlsl" | "wgsl" | "d" | "v" | "verilog" | "apex" => "clike",
        "scss" | "less" | "sass" | "stylus" => "scss",
        "graphql" | "gql" => "graphql",
        "erlang" | "erl" | "matlab" | "octave" | "latex" | "tex" => "erlang",
        "lisp" | "clojure" | "clj" | "scheme" | "racket" | "elisp" | "asm" | "nasm" | "ini2" => "lisp",
        _ => return None,
    })
}

fn skip_string(code: &[u8], mut i: usize, s: &StrSpec) -> usize {
    let close = s.close.as_bytes();
    while i < code.len() {
        if s.esc == Some(code[i]) {
            i += 2;
            continue;
        }
        if code[i..].starts_with(close) {
            if s.doubled && code[i + close.len()..].starts_with(close) {
                i += close.len() * 2;
                continue;
            }
            return i + close.len();
        }
        // Single-line strings end at line breaks; this keeps a stray quote from swallowing the file.
        if code[i] == b'\n' && s.open.len() == 1 && s.open != "`" && s.open != "[" {
            return i;
        }
        i += 1;
    }
    i.min(code.len())
}

fn lex_comments(code: &str, spec: &LexSpec) -> Vec<CommentRange> {
    let b = code.as_bytes();
    let mut out = Vec::new();
    let mut at_line_start = true;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            at_line_start = true;
            i += 1;
            continue;
        }
        if let Some((open, close)) = spec.block.iter().find(|(o, _)| b[i..].starts_with(o.as_bytes())) {
            if !spec.line_start_only || at_line_start {
                let from = i + open.len();
                let end = code[from..].find(close).map(|p| from + p + close.len()).unwrap_or(b.len());
                out.push(CommentRange { start: i, end });
                i = end;
                at_line_start = false;
                continue;
            }
        }
        let line = spec.line.iter().find(|o| b[i..].starts_with(o.as_bytes()));
        let space_ok = !spec.line_needs_space || i == 0 || code[..i].chars().next_back().is_some_and(jsstr::is_js_space);
        if line.is_some() && (!spec.line_start_only || at_line_start) && space_ok {
            let mut end = code[i..].find('\n').map(|p| p + i).unwrap_or(b.len());
            if end > 0 && b[end - 1] == b'\r' {
                end -= 1;
            }
            out.push(CommentRange { start: i, end });
            i = end;
            continue;
        }
        if let Some(st) = spec.strings.iter().find(|s| b[i..].starts_with(s.open.as_bytes())) {
            i = skip_string(b, i + st.open.len(), st);
            at_line_start = false;
            continue;
        }
        let ch = code[i..].chars().next().unwrap();
        if !jsstr::is_js_space(ch) {
            at_line_start = false;
        }
        i += ch.len_utf8();
    }
    out
}

/// Comment ranges in `code`, or `None` when the language is not supported (the block is then left untouched).
pub fn find_comments(lang: &str, code: &str) -> Option<Vec<CommentRange>> {
    let key = lang_key(lang);
    if let Some(grammar) = grammar_for(&key) {
        return Some(tree_sitter_comments(grammar, code));
    }
    let spec = lex_spec(lex_alias(&key)?)?;
    Some(lex_comments(code, spec))
}

rx!(WS_RUN, r"\s+");

/// Code with comments (and docstrings) removed and whitespace collapsed; proves code bytes were not altered.
pub fn code_fingerprint(lang: &str, code: &str) -> Option<String> {
    let found = find_comments(lang, code)?;
    let mut comments = found;
    comments.extend(find_docstrings(lang, code));
    comments.sort_by_key(|c| c.start);
    let mut out = String::new();
    let mut pos = 0;
    for c in &comments {
        if c.start < pos {
            continue;
        }
        out.push_str(&code[pos..c.start]);
        out.push(' ');
        pos = c.end;
    }
    out.push_str(&code[pos..]);
    let collapsed = WS_RUN().replace_all_str(&out, " ");
    Some(format!("{}\u{0}{}", jsstr::trim(&collapsed), if comments.is_empty() { "" } else { "c" }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts<'a>(code: &'a str, r: &[CommentRange]) -> Vec<&'a str> {
        r.iter().map(|c| &code[c.start..c.end]).collect()
    }

    #[test]
    fn tree_sitter_languages() {
        let code = "// one\nconst a = \"// no\"; /* two */\n";
        assert_eq!(texts(code, &find_comments("typescript", code).unwrap()), vec!["// one", "/* two */"]);
        let py = "def f():\n    \"\"\"Doc.\"\"\"\n    return 1  # tail\n";
        assert_eq!(texts(py, &find_comments("python", py).unwrap()), vec!["# tail"]);
        assert_eq!(texts(py, &find_docstrings("python", py)), vec!["\"\"\"Doc.\"\"\""]);
        let rs = "/// Doc comment\nfn main() {}\n";
        assert_eq!(texts(rs, &find_comments("rust", rs).unwrap()), vec!["/// Doc comment"]);
    }

    #[test]
    fn lexer_languages() {
        let sql = "SELECT '--x' -- real\nFROM t /* b */";
        assert_eq!(texts(sql, &find_comments("sql", sql).unwrap()), vec!["-- real", "/* b */"]);
        let yaml = "a: b # c\nurl: http://x#y\n";
        assert_eq!(texts(yaml, &find_comments("yaml", yaml).unwrap()), vec!["# c"]);
        assert!(find_comments("json", "{}").is_none());
    }

    #[test]
    fn parses_as_code_checks() {
        assert_eq!(parses_as_code("js", "const a = 1;"), Some(true));
        assert_eq!(parses_as_code("python", "This is prose, not code."), Some(false));
        assert_eq!(parses_as_code("json", "{}"), None);
    }

    #[test]
    fn lang_keys() {
        assert_eq!(lang_key("{.python startFrom=1}"), "python");
        assert_eq!(lang_key(" TS,linenos"), "ts");
    }
}
