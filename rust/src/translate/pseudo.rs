//! Deterministic pseudo translation (`test/helpers.ts`): every word gets a prefix and is upper-cased; tags stay
//! where they are. Used by the golden tests and `-engine pseudo`.

use crate::mask::masking::{tokenize, xml_escape, xml_unescape, MaskToken};
use crate::types::{Extraction, TMap};
use regex::Regex;
use std::sync::OnceLock;

pub fn pseudo_words(s: &str) -> String {
    static WORD: OnceLock<Regex> = OnceLock::new();
    WORD.get_or_init(|| Regex::new(r"\p{L}+").unwrap()).replace_all(s, |c: &regex::Captures| format!("Ü{}", c[0].to_uppercase())).into_owned()
}

pub fn pseudo(masked: &str) -> String {
    tokenize(masked)
        .into_iter()
        .map(|t| match t {
            MaskToken::Text(v) => xml_escape(&pseudo_words(&xml_unescape(&v))),
            MaskToken::X(n) => format!("<x{n}/>"),
            MaskToken::Open(n) => format!("<g{n}>"),
            MaskToken::Close(n) => format!("</g{n}>"),
        })
        .collect()
}

pub fn pseudo_map(ex: &Extraction) -> TMap {
    ex.active().map(|s| (s.id.clone(), pseudo(&s.masked))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_tags() {
        assert_eq!(pseudo("Hello <g1>wörld</g1> &amp; <x2/> straße"), "ÜHELLO <g1>ÜWÖRLD</g1> &amp; <x2/> ÜSTRASSE");
    }
}
