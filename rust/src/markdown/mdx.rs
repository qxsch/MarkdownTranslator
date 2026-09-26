//! Validation of MDX JavaScript (expressions and ESM) with a real parser, like micromark-extension-mdxjs does
//! with acorn in the TypeScript implementation: invalid expressions such as `{\frac{a}{b}}` or `{< shortcode >}`
//! make the document fail instead of being accepted by brace matching.

use markdown::{MdxExpressionKind, MdxSignal};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_parser::Parser;
use oxc_span::SourceType;

fn source_type() -> SourceType {
    SourceType::mjs().with_jsx(true)
}

fn error(message: impl Into<String>, offset: usize) -> MdxSignal {
    MdxSignal::Error(message.into(), offset, Box::new("mdtranslate".into()), Box::new("acorn".into()))
}

fn eof(message: impl Into<String>) -> MdxSignal {
    MdxSignal::Eof(message.into(), Box::new("mdtranslate".into()), Box::new("acorn".into()))
}

/// Only whitespace and comments (allowed as `{/* note */}` in prose).
fn is_empty_code(code: &str) -> bool {
    let b = code.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
        } else if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            match code[i + 2..].find("*/") {
                Some(p) => i += p + 4,
                None => return false,
            }
        } else {
            return false;
        }
    }
    true
}

/// Brackets, strings, template literals or block comments still open at the end: more lines may complete it.
fn is_open(value: &str) -> bool {
    #[derive(PartialEq)]
    enum S {
        Code,
        Single,
        Double,
        Template,
        LineComment,
        BlockComment,
    }
    let b = value.as_bytes();
    let mut stack = vec![S::Code];
    let mut depth = 0i64;
    let mut template_depths: Vec<i64> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match stack.last().unwrap() {
            S::Code => match c {
                b'\'' => stack.push(S::Single),
                b'"' => stack.push(S::Double),
                b'`' => stack.push(S::Template),
                b'/' if b.get(i + 1) == Some(&b'/') => {
                    stack.push(S::LineComment);
                    i += 1;
                }
                b'/' if b.get(i + 1) == Some(&b'*') => {
                    stack.push(S::BlockComment);
                    i += 1;
                }
                b'{' | b'(' | b'[' => depth += 1,
                b'}' if template_depths.last() == Some(&depth) => {
                    template_depths.pop();
                    stack.pop();
                }
                b'}' | b')' | b']' => depth -= 1,
                _ => {}
            },
            s @ (S::Single | S::Double) => {
                let q = if *s == S::Single { b'\'' } else { b'"' };
                if c == b'\\' {
                    i += 1;
                } else if c == q || c == b'\n' {
                    stack.pop();
                }
            }
            S::Template => {
                if c == b'\\' {
                    i += 1;
                } else if c == b'`' {
                    stack.pop();
                } else if c == b'$' && b.get(i + 1) == Some(&b'{') {
                    template_depths.push(depth);
                    stack.push(S::Code);
                    i += 1;
                }
            }
            S::LineComment => {
                if c == b'\n' {
                    stack.pop();
                }
            }
            S::BlockComment => {
                if c == b'*' && b.get(i + 1) == Some(&b'/') {
                    stack.pop();
                    i += 1;
                }
            }
        }
        i += 1;
    }
    depth > 0 || stack.iter().skip(1).any(|s| !matches!(s, S::LineComment))
}

/// `mdx_expression_parse`: `{expression}` in prose, `{...spread}` and `attr={value}` in JSX.
pub fn expression_signal(value: &str, kind: &MdxExpressionKind) -> MdxSignal {
    let allocator = Allocator::default();
    match kind {
        MdxExpressionKind::Expression => {
            if is_empty_code(value) {
                return MdxSignal::Ok;
            }
            match Parser::new(&allocator, value, source_type()).parse_expression() {
                Ok(_) => MdxSignal::Ok,
                Err(d) => error(format!("Could not parse expression with acorn: {}", d.first().map(|e| e.to_string()).unwrap_or_default()), 0),
            }
        }
        MdxExpressionKind::AttributeValueExpression => {
            if is_empty_code(value) {
                return error("Unexpected empty expression, expected a value", 0);
            }
            match Parser::new(&allocator, value, source_type()).parse_expression() {
                Ok(_) => MdxSignal::Ok,
                Err(d) => error(format!("Could not parse expression with acorn: {}", d.first().map(|e| e.to_string()).unwrap_or_default()), 0),
            }
        }
        MdxExpressionKind::AttributeExpression => {
            let wrapped = format!("({{{value}}})");
            match Parser::new(&allocator, &wrapped, source_type()).parse_expression() {
                Ok(expr) => {
                    let expr = match expr {
                        Expression::ParenthesizedExpression(p) => p.unbox().expression,
                        e => e,
                    };
                    match expr {
                        Expression::ObjectExpression(o) if o.properties.len() == 1 && matches!(o.properties[0], ObjectPropertyKind::SpreadProperty(_)) => MdxSignal::Ok,
                        _ => error("Unexpected extra content in spread: only a single spread is supported", 0),
                    }
                }
                Err(d) => error(format!("Could not parse expression with acorn: {}", d.first().map(|e| e.to_string()).unwrap_or_default()), 0),
            }
        }
    }
}

/// `mdx_esm_parse`: import/export statements; the block continues past blank lines while it is incomplete.
pub fn esm_signal(value: &str) -> MdxSignal {
    if is_open(value) {
        return eof("Unexpected end of file in expression, expected a corresponding closing brace for `{`");
    }
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, value, source_type()).parse();
    if let Some(first) = ret.diagnostics.first() {
        // An error at the very end means the code is incomplete (more lines may follow), like acorn's `eof`.
        let end = value.trim_end().len();
        let at_end = ret.diagnostics.iter().any(|d| d.labels.iter().any(|l| l.offset() as usize >= end));
        return if at_end { eof(format!("Could not parse import/exports with acorn: {first}")) } else { error(format!("Could not parse import/exports with acorn: {first}"), 0) };
    }
    for stmt in &ret.program.body {
        if !stmt.is_module_declaration() {
            return error("Unexpected statement in code: only import/exports are supported", 0);
        }
    }
    MdxSignal::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: MdxSignal) -> bool {
        matches!(s, MdxSignal::Ok)
    }

    #[test]
    fn expressions() {
        assert!(ok(expression_signal("props.title", &MdxExpressionKind::Expression)));
        assert!(ok(expression_signal("items.map((i) => <li key={i}>{i}</li>)", &MdxExpressionKind::Expression)));
        assert!(ok(expression_signal(" /* comment */ ", &MdxExpressionKind::Expression)));
        assert!(!ok(expression_signal("\\frac{a}{b}", &MdxExpressionKind::Expression)));
        assert!(!ok(expression_signal("< figure src=\"x\" >", &MdxExpressionKind::Expression)));
        assert!(!ok(expression_signal("b_{t-1}", &MdxExpressionKind::Expression)));
        assert!(ok(expression_signal("...props", &MdxExpressionKind::AttributeExpression)));
        assert!(!ok(expression_signal("a", &MdxExpressionKind::AttributeExpression)));
        assert!(!ok(expression_signal("", &MdxExpressionKind::AttributeValueExpression)));
    }

    #[test]
    fn esm() {
        assert!(ok(esm_signal("import X from 'y'")));
        let meta = esm_signal("export const meta = { title: 'A' }");
        assert!(ok(meta.clone()), "{meta:?}");
        assert!(matches!(esm_signal("export const a = {"), MdxSignal::Eof(..)));
        assert!(matches!(esm_signal("export const a ="), MdxSignal::Eof(..)));
        assert!(matches!(esm_signal("import X from 'y'\nconsole.log(X)"), MdxSignal::Error(..)));
    }
}
