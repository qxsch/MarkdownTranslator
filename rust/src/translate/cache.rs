//! Segment-level translation memory: in-memory LRU with optional file persistence. Port of
//! `translate/cache.ts`; keys and files use the same format, so a cache directory can be shared.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct TranslationCache {
    dir: Option<PathBuf>,
    max_entries: usize,
    mem: Mutex<(HashMap<String, String>, VecDeque<String>)>,
}

impl TranslationCache {
    pub fn new(dir: Option<PathBuf>) -> Self {
        TranslationCache { dir, max_entries: 200_000, mem: Mutex::new((HashMap::new(), VecDeque::new())) }
    }

    /// `sha256(JSON.stringify(parts))`.
    pub fn key(parts: &Value) -> String {
        let json = serde_json::to_string(parts).expect("serializable");
        let digest = Sha256::digest(json.as_bytes());
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(&key[..2]).join(format!("{key}.json")))
    }

    fn remember(&self, key: &str, value: &str) {
        let mut guard = self.mem.lock().unwrap();
        let (map, order) = &mut *guard;
        if map.insert(key.to_string(), value.to_string()).is_none() {
            order.push_back(key.to_string());
        }
        while map.len() > self.max_entries {
            match order.pop_front() {
                Some(old) => {
                    map.remove(&old);
                }
                None => break,
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        if let Some(hit) = self.mem.lock().unwrap().0.get(key).cloned() {
            return Some(hit);
        }
        let path = self.path(key)?;
        let text = std::fs::read_to_string(path).ok()?;
        let v: Value = serde_json::from_str(&text).ok()?;
        let t = v.get("t")?.as_str()?.to_string();
        self.remember(key, &t);
        Some(t)
    }

    pub fn set(&self, key: &str, value: &str) {
        self.remember(key, value);
        if let Some(path) = self.path(key) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, serde_json::json!({ "t": value }).to_string());
        }
    }
}
