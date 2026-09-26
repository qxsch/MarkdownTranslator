//! Golden-file parity with the TypeScript implementation: every fixture is extracted, pseudo-translated and
//! assembled, and the result must equal what `rust/tools/golden.ts` recorded from the TypeScript code.

use mdtranslate::dump::{golden, Variant};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn do_not_translate(root: &Path) -> Vec<String> {
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(root.join("config/glossary.json")).unwrap()).unwrap();
    raw["doNotTranslate"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn variant(o: &Value) -> Variant {
    let b = |k: &str| o[k].as_bool().unwrap_or(false);
    Variant {
        mdx: b("mdx"),
        math_single_dollar: b("mathSingleDollar"),
        docstrings: b("docstrings"),
        code_comments: b("codeComments"),
        front_matter: b("frontMatter"),
        preserve_anchors: b("preserveAnchors"),
    }
}

/// First difference between two JSON values, as a readable path.
fn diff(path: &str, a: &Value, b: &Value) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                if let Some(d) = diff(&format!("{path}.{k}"), v, y.get(k).unwrap_or(&Value::Null)) {
                    return Some(d);
                }
            }
            y.keys().find(|k| !x.contains_key(*k)).map(|k| format!("{path}.{k}: unexpected key"))
        }
        (Value::Array(x), Value::Array(y)) => {
            for (i, (v, w)) in x.iter().zip(y.iter()).enumerate() {
                if let Some(d) = diff(&format!("{path}[{i}]"), v, w) {
                    return Some(d);
                }
            }
            (x.len() != y.len()).then(|| format!("{path}: length {} (TypeScript) vs {} (Rust)", x.len(), y.len()))
        }
        _ if a == b => None,
        _ => Some(format!("{path}:\n    TypeScript: {a}\n    Rust:       {b}")),
    }
}

/// Error texts differ between the implementations; only the failing stage has to match.
fn normalize(mut v: Value) -> Value {
    if let Some(e) = v.get("error").and_then(|e| e.as_str()).map(|e| e.split(':').next().unwrap_or("").to_string()) {
        v["error"] = Value::String(e);
    }
    if let Some(notes) = v.get_mut("notes").and_then(|n| n.as_array_mut()) {
        for n in notes.iter_mut() {
            if let Some(s) = n.as_str() {
                if let Some(i) = s.find("(YAML error: ") {
                    *n = Value::String(format!("{}(YAML error)", &s[..i]));
                }
            }
        }
    }
    v
}

#[test]
fn golden_files_match_typescript() {
    let root = repo_root();
    let dnt = do_not_translate(&root);
    let dir = root.join("rust/tests/golden");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
    files.sort();
    assert!(!files.is_empty(), "no golden files in {}", dir.display());
    let only = std::env::var("GOLDEN_ONLY").ok();
    let mut failures = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if only.as_ref().is_some_and(|o| !name.contains(o.as_str())) {
            continue;
        }
        let expected: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let src = std::fs::read_to_string(root.join(expected["file"].as_str().unwrap())).unwrap();
        let mut actual = golden(&src, &dnt, &variant(&expected["options"]));
        let mut expected = expected;
        let obj = expected.as_object_mut().unwrap();
        // shift_remove keeps the key order (remove is swap_remove with preserve_order), so the first reported
        // difference is the first one in document order.
        obj.shift_remove("file");
        obj.shift_remove("options");
        if obj.shift_remove("sameExtractionAsDefault").is_some() {
            let keep = ["output", "reverted", "anchors", "error"];
            actual.as_object_mut().unwrap().retain(|k, _| keep.contains(&k.as_str()));
        }
        if let Some(d) = diff("", &normalize(expected), &normalize(actual)) {
            failures.push(format!("{name}{d}"));
        }
    }
    if !failures.is_empty() {
        panic!("{} of {} golden files differ:\n\n{}", failures.len(), files.len(), failures.join("\n\n"));
    }
}
