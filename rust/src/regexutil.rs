//! JavaScript-compatible regular expressions.
//!
//! The port carries roughly 190 regular expressions over from the TypeScript original. Several dialect
//! differences would silently change behaviour if they were ignored:
//!
//! * In JavaScript `\w`, `\d` and `\b` are ASCII-only, while Rust's `regex` crate makes them Unicode-aware
//!   (`\bfoo\b` next to `é` matches in Rust but not in JavaScript).
//! * JavaScript's `\s` includes U+FEFF and excludes U+0085; `.` excludes `\r`, U+2028 and U+2029.
//! * A `[` inside a character class is literal in JavaScript but starts a nested class in Rust, and
//!   `&&`, `--`, `~~` are set operations there.
//!
//! [`translate`] rewrites a JavaScript pattern into an equivalent `fancy-regex` pattern: ASCII classes are
//! spelled out, `\b` becomes the lookaround pair that defines it, and literals that are special only in
//! Rust are escaped.

use fancy_regex::Regex;
use std::sync::OnceLock;

const WORD: &str = "0-9A-Za-z_";
/// Members of JavaScript's `\s` class.
const SPACE: &str = r"\t\n\x0B\x0C\r \u{A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";
/// JavaScript `\b`: a position with a word character on exactly one side.
const BOUNDARY: &str = "(?:(?<=[0-9A-Za-z_])(?![0-9A-Za-z_])|(?<![0-9A-Za-z_])(?=[0-9A-Za-z_]))";
/// JavaScript `\B`.
const NOT_BOUNDARY: &str = "(?:(?<=[0-9A-Za-z_])(?=[0-9A-Za-z_])|(?<![0-9A-Za-z_])(?![0-9A-Za-z_]))";

/// Rewrites a JavaScript regular expression source (with its flags) into a `fancy-regex` source.
pub fn translate_with_flags(pattern: &str, flags: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let dot_all = flags.contains('s');
    let mut out = String::with_capacity(pattern.len() + 32);
    let mut inline = String::new();
    if flags.contains('i') {
        inline.push('i');
    }
    if flags.contains('m') {
        inline.push('m');
    }
    if !inline.is_empty() {
        out.push_str("(?");
        out.push_str(&inline);
        out.push(')');
    }
    let mut i = 0;
    let mut in_class = false;
    // Position right after the opening `[` (or `[^`) of the current class.
    let mut class_start = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let n = chars[i + 1];
            i += 2;
            match n {
                'w' if in_class => out.push_str(WORD),
                'w' => out.push_str("[0-9A-Za-z_]"),
                'W' if in_class => out.push_str("[^0-9A-Za-z_]"),
                'W' => out.push_str("[^0-9A-Za-z_]"),
                'd' if in_class => out.push_str("0-9"),
                'd' => out.push_str("[0-9]"),
                'D' if in_class => out.push_str("[^0-9]"),
                'D' => out.push_str("[^0-9]"),
                's' if in_class => out.push_str(SPACE),
                's' => {
                    out.push('[');
                    out.push_str(SPACE);
                    out.push(']');
                }
                'S' => {
                    out.push_str("[^");
                    out.push_str(SPACE);
                    out.push(']');
                }
                'b' if in_class => out.push_str(r"\x08"),
                'b' => out.push_str(BOUNDARY),
                'B' => out.push_str(NOT_BOUNDARY),
                'u' if i < chars.len() && chars[i] == '{' => {
                    // \u{XXXX} is the same in both dialects.
                    out.push_str("\\u");
                    while i < chars.len() {
                        out.push(chars[i]);
                        i += 1;
                        if chars[i - 1] == '}' {
                            break;
                        }
                    }
                }
                'u' if i + 4 <= chars.len() && chars[i..i + 4].iter().all(|c| c.is_ascii_hexdigit()) => {
                    out.push_str("\\u{");
                    out.extend(&chars[i..i + 4]);
                    out.push('}');
                    i += 4;
                }
                'p' | 'P' => {
                    // \p{L}: the property name in braces is copied as is.
                    out.push('\\');
                    out.push(n);
                    if i < chars.len() && chars[i] == '{' {
                        while i < chars.len() {
                            out.push(chars[i]);
                            i += 1;
                            if chars[i - 1] == '}' {
                                break;
                            }
                        }
                    }
                }
                'x' | 'n' | 'r' | 't' | 'f' | 'v' | '0'..='9' => {
                    if n == 'v' {
                        out.push_str(r"\x0B");
                    } else {
                        out.push('\\');
                        out.push(n);
                    }
                }
                _ if n.is_ascii_alphanumeric() => {
                    out.push('\\');
                    out.push(n);
                }
                _ => push_literal(&mut out, n, in_class),
            }
            continue;
        }
        if in_class {
            match c {
                ']' => {
                    in_class = false;
                    out.push(']');
                }
                '[' | '&' | '~' => {
                    out.push('\\');
                    out.push(c);
                }
                '-' if out.len() == class_start || chars.get(i + 1) == Some(&']') => out.push_str(r"\-"),
                '-' if chars.get(i + 1) == Some(&'-') => out.push_str(r"\-"),
                _ => out.push(c),
            }
            i += 1;
            continue;
        }
        match c {
            '[' => {
                in_class = true;
                out.push('[');
                if chars.get(i + 1) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                class_start = out.len();
            }
            '.' if !dot_all => out.push_str(r"[^\n\r\u{2028}\u{2029}]"),
            '.' => out.push_str(r"(?s:.)"),
            '{' if !is_quantifier(&chars, i) => out.push_str(r"\{"),
            '}' if !closes_quantifier(&chars, i) => out.push_str(r"\}"),
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

/// Rewrites a JavaScript regular expression source without flags.
pub fn translate(pattern: &str) -> String {
    translate_with_flags(pattern, "")
}

fn push_literal(out: &mut String, c: char, in_class: bool) {
    let special = if in_class { "\\]^-[&~" } else { "\\.+*?()|[]{}^$#&-~" };
    if special.contains(c) {
        out.push('\\');
    }
    out.push(c);
}

/// Is the `{` at `i` the start of a `{n}`, `{n,}` or `{n,m}` quantifier?
fn is_quantifier(chars: &[char], i: usize) -> bool {
    let mut j = i + 1;
    let digits = |j: &mut usize| {
        let s = *j;
        while *j < chars.len() && chars[*j].is_ascii_digit() {
            *j += 1;
        }
        *j > s
    };
    if !digits(&mut j) {
        return false;
    }
    if j < chars.len() && chars[j] == ',' {
        j += 1;
        digits(&mut j);
    }
    j < chars.len() && chars[j] == '}'
}

fn closes_quantifier(chars: &[char], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        j -= 1;
        match chars[j] {
            '{' => return is_quantifier(chars, j),
            c if c.is_ascii_digit() || c == ',' => continue,
            _ => return false,
        }
    }
    false
}

/// Compiles a JavaScript regular expression source. Panics on an invalid pattern, which can only be a bug
/// in a literal inside this crate.
pub fn js(pattern: &str) -> Regex {
    jsf(pattern, "")
}

/// Compiles a JavaScript regular expression source with JavaScript flags (`i`, `m`, `s`; `g`/`u`/`y` are
/// implied by the call sites).
pub fn jsf(pattern: &str, flags: &str) -> Regex {
    let translated = translate_with_flags(pattern, flags);
    Regex::new(&translated).unwrap_or_else(|e| panic!("invalid regex {pattern:?} (translated to {translated:?}): {e}"))
}

/// Lazily compiled JavaScript regular expression literal: `rx!(NAME, r"pattern")` or
/// `rx!(NAME, r"pattern", "i")` defines `fn NAME() -> &'static Regex`.
#[macro_export]
macro_rules! rx {
    ($name:ident, $pattern:expr) => {
        $crate::rx!($name, $pattern, "");
    };
    ($name:ident, $pattern:expr, $flags:expr) => {
        #[allow(non_snake_case)]
        fn $name() -> &'static fancy_regex::Regex {
            static CELL: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
            CELL.get_or_init(|| $crate::regexutil::jsf($pattern, $flags))
        }
    };
}

/// One match: the full range plus the capture groups, indexed like JavaScript's `RegExpExecArray`.
#[derive(Clone, Debug)]
pub struct Match<'t> {
    pub start: usize,
    pub end: usize,
    pub text: &'t str,
    pub groups: Vec<Option<(usize, usize, &'t str)>>,
}

impl<'t> Match<'t> {
    /// Capture group `n`, or `None` when it did not participate.
    pub fn group(&self, n: usize) -> Option<&'t str> {
        self.groups.get(n).and_then(|g| g.map(|(_, _, s)| s))
    }

    /// Capture group `n`, or the empty string (JavaScript's `m[n] ?? ''`).
    pub fn g(&self, n: usize) -> &'t str {
        self.group(n).unwrap_or("")
    }

    pub fn group_start(&self, n: usize) -> Option<usize> {
        self.groups.get(n).and_then(|g| g.map(|(s, _, _)| s))
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

/// Convenience wrappers that hide `fancy-regex`'s fallible API. A match failure can only come from
/// backtracking limits, which the patterns in this crate do not hit; it is treated as "no match".
pub trait RegexExt {
    fn test(&self, text: &str) -> bool;
    /// Every non-overlapping match (JavaScript `matchAll` / a global `exec` loop).
    fn all<'t>(&self, text: &'t str) -> Vec<Match<'t>>;
    /// First match (JavaScript `exec` without the global flag).
    fn exec<'t>(&self, text: &'t str) -> Option<Match<'t>>;
    /// First match at or after byte offset `pos`, looking behind `pos` for lookbehinds (a global `exec`).
    fn exec_at<'t>(&self, text: &'t str, pos: usize) -> Option<Match<'t>>;
    /// `text.replace(re, f)` for a global regular expression.
    fn replace_all_with(&self, text: &str, f: impl FnMut(&Match) -> String) -> String;
    /// `text.replace(re, replacement)` for a global regular expression with a literal replacement.
    fn replace_all_str(&self, text: &str, replacement: &str) -> String;
}

impl RegexExt for Regex {
    fn test(&self, text: &str) -> bool {
        self.is_match(text).unwrap_or(false)
    }

    fn all<'t>(&self, text: &'t str) -> Vec<Match<'t>> {
        let mut out = Vec::new();
        let mut pos = 0usize;
        while pos <= text.len() {
            match self.captures_from_pos(text, pos) {
                Ok(Some(caps)) => {
                    let m = to_match(&caps);
                    // Zero-length matches must still advance, by one character.
                    pos = if m.end > m.start { m.end } else { next_char_boundary(text, m.end) };
                    out.push(m);
                }
                _ => break,
            }
        }
        out
    }

    fn exec<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        match self.captures(text) {
            Ok(Some(caps)) => Some(to_match(&caps)),
            _ => None,
        }
    }

    fn exec_at<'t>(&self, text: &'t str, pos: usize) -> Option<Match<'t>> {
        if pos > text.len() {
            return None;
        }
        match self.captures_from_pos(text, pos) {
            Ok(Some(caps)) => Some(to_match(&caps)),
            _ => None,
        }
    }

    fn replace_all_with(&self, text: &str, mut f: impl FnMut(&Match) -> String) -> String {
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for m in self.all(text) {
            out.push_str(&text[last..m.start]);
            out.push_str(&f(&m));
            last = m.end;
        }
        out.push_str(&text[last..]);
        out
    }

    fn replace_all_str(&self, text: &str, replacement: &str) -> String {
        self.replace_all_with(text, |_| replacement.to_string())
    }
}

fn to_match<'t>(caps: &fancy_regex::Captures<'t, str>) -> Match<'t> {
    let whole = caps.get(0).expect("group 0 always participates");
    let groups = (0..caps.len()).map(|i| caps.get(i).map(|m| (m.start(), m.end(), m.as_str()))).collect();
    Match { start: whole.start(), end: whole.end(), text: whole.as_str(), groups }
}

fn next_char_boundary(text: &str, from: usize) -> usize {
    let mut i = from + 1;
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Escapes a literal for a JavaScript pattern, like the `esc` helpers in the TypeScript code
/// (`s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')`).
pub fn escape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if ".*+?^${}()|[]\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `/\p{L}/u.test(s)`: does the text contain a letter?
pub fn has_letter(s: &str) -> bool {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\p{L}").unwrap()).test(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_classes_stay_ascii() {
        let re = js(r"\bfoo\b");
        assert!(re.test("a foo b"));
        // JavaScript sees a boundary between "é" (not a word character there) and "f"; Unicode \b does not.
        assert!(re.test("éfoo"));
        assert!(!js(r"\w").test("é"));
    }

    #[test]
    fn classes_inside_brackets() {
        let re = js(r"[\w.-]+");
        assert_eq!(re.exec("a_b.c-d!").unwrap().text, "a_b.c-d");
        assert_eq!(re.exec("éx").unwrap().text, "x");
        let neg = js(r"[\W\d]+");
        assert_eq!(neg.exec("ab 12 cd").unwrap().text, " 12 ");
    }

    #[test]
    fn literal_bracket_in_class() {
        let re = js(r"\\[!-/:-@[-`{-~]");
        assert!(re.test(r"\["));
        assert!(re.test(r"\_"));
        assert!(!re.test(r"\a"));
    }

    #[test]
    fn js_dot_and_space() {
        assert!(!js(r"^a.b$").test("a\rb"));
        assert!(js(r"^a.b$", ).test("a b"));
        assert!(js(r"\s").test("\u{FEFF}"));
        assert!(!js(r"\s").test("\u{85}"));
    }

    #[test]
    fn lookaround_and_backrefs() {
        let re = js(r"(?<![\w-])--[A-Za-z0-9][\w-]*");
        assert_eq!(re.exec("run --verbose now").unwrap().text, "--verbose");
        assert!(re.exec("a---b").is_none());
        let br = js(r#"(["'])(.*?)\1"#);
        assert_eq!(br.exec(r#"x="a'b" y"#).unwrap().g(2), "a'b");
    }

    #[test]
    fn braces_and_escapes() {
        assert!(js(r"^\{\{[<%]").test("{{< note >}}"));
        assert!(js(r"x{2,3}").test("xxx"));
        assert!(js(r"a{b").test("a{b"));
        assert!(js(r"\/path").test("/path"));
        assert!(jsf(r"^abc$", "i").test("ABC"));
        assert!(js(r"^\p{L}{3,}$").test("äbc"));
        assert!(!js(r"^\p{L}{3,}$").test("ab"));
        assert!(js(r"\u{1F600}|\u00e9").test("é"));
        assert!(js(r"[\p{L}\p{N}]").test("5"));
    }

    #[test]
    fn zero_length_matches_terminate() {
        assert!(js(r"x*").all("axb").len() >= 3);
    }

    /// Every `rx!` literal in the crate compiles (they are compiled lazily, so a broken one would only fail at
    /// first use).
    #[test]
    fn all_regex_literals_compile() {
        let finder = regex::Regex::new(r#"(?s)rx!\(\s*\w+,\s*r(#*)"(.*?)"(#*)\s*(?:,\s*"(\w*)")?\s*\)"#).unwrap();
        let mut stack = vec![std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))];
        let mut count = 0;
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") || path.ends_with("regexutil.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for c in finder.captures_iter(&text) {
                    if c[1] != c[3] {
                        continue;
                    }
                    let flags = c.get(4).map(|m| m.as_str()).unwrap_or("");
                    let translated = translate_with_flags(&c[2], flags);
                    if let Err(e) = Regex::new(&translated) {
                        panic!("{}: {:?} -> {:?}: {e}", path.display(), &c[2], translated);
                    }
                    count += 1;
                }
            }
        }
        assert!(count > 90, "found only {count} regex literals");
    }
}
