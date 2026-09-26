/**
 * Single-dollar inline math, detected conservatively so prices such as "$5 and $10" stay prose.
 * Port of `mathSpans.ts`: Pandoc's rules (the opening `$` is followed by a non-space, the closing `$` follows
 * a non-space and is not followed by a digit) plus a check that the content looks like a formula.
 */
use crate::regexutil::RegexExt;
use crate::rx;

rx!(TEXT_COMMAND, r"\\(?:text|textrm|mathrm|operatorname|mbox)\{[^{}]*\}");
rx!(PLAIN_WORD, r"(?<![\\A-Za-z])[A-Za-z]{2,}");
rx!(MATH_SIGNAL, r"\\[A-Za-z]+|[\^_=<>{}|]");
rx!(SINGLE_VARIABLE, r"^[A-Za-z](?:_?\d+)?'*$");
rx!(BINARY_OP, r"^[A-Za-z0-9.]+\s*[-+*/\u00d7\u00b7]\s*[A-Za-z0-9.(]");

fn is_escaped(s: &[u8], i: usize) -> bool {
    let mut n = 0;
    while i >= n + 1 && s[i - 1 - n] == b'\\' {
        n += 1;
    }
    n % 2 == 1
}

pub fn looks_like_math(content: &str) -> bool {
    let body = TEXT_COMMAND().replace_all_str(content, " ");
    // Three or more plain words (not LaTeX commands) mean prose between two dollar amounts.
    if PLAIN_WORD().all(&body).len() >= 3 {
        return false;
    }
    let t = crate::jsstr::trim(content);
    MATH_SIGNAL().test(t) || SINGLE_VARIABLE().test(t) || BINARY_OP().test(t)
}

fn is_space_at(text: &str, i: usize) -> bool {
    text[i..].chars().next().is_some_and(crate::jsstr::is_js_space)
}

fn is_space_before(text: &str, i: usize) -> bool {
    text[..i].chars().next_back().is_some_and(crate::jsstr::is_js_space)
}

/// Byte ranges of `$...$` spans (dollars included) in `text` that are inline math.
pub fn find_dollar_math(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if b[i] != b'$' || (i + 1 < n && b[i + 1] == b'$') || (i > 0 && b[i - 1] == b'$') || is_escaped(b, i) {
            i += 1;
            continue;
        }
        if i + 1 >= n || is_space_at(text, i + 1) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < n && !(b[j] == b'$' && !is_escaped(b, j)) && !(b[j] == b'\n' && j + 1 < n && b[j + 1] == b'\n') {
            j += 1;
        }
        if j >= n || b[j] != b'$' || (j + 1 < n && b[j + 1] == b'$') {
            i += 1;
            continue;
        }
        let next_is_digit = j + 1 < n && b[j + 1].is_ascii_digit();
        if is_space_before(text, j) || next_is_digit || !looks_like_math(&text[i + 1..j]) {
            i += 1;
            continue;
        }
        out.push((i, j + 1));
        i = j + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_stay_prose() {
        assert!(find_dollar_math("It costs $5 and $10 per month.").is_empty());
        assert!(find_dollar_math("Pay $5, get $10 back").is_empty());
    }

    #[test]
    fn formulas_are_math() {
        assert_eq!(find_dollar_math(r"where $\alpha = 0.7$ holds"), vec![(6, 20)]);
        assert_eq!(find_dollar_math("let $x$ be"), vec![(4, 7)]);
        assert_eq!(find_dollar_math("and $a + b$ ok"), vec![(4, 11)]);
    }
}
