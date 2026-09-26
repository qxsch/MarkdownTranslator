//! Command line contract: stdout carries only the output of `-targetFile -` (and `-help`); progress and
//! errors go to stderr.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_mdtranslate");

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/identity.md")
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .env_remove("AZURE_OPENAI_ENDPOINT")
        .env_remove("AZURE_TRANSLATOR_ENDPOINT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary runs");
    if let Some(input) = stdin {
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().unwrap()
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mdtranslate-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn translation_to_a_file_keeps_stdout_empty() {
    let out = tmp("identity.de.md");
    let src = fixture();
    let o = run(&["-sourceFile", src.to_str().unwrap(), "-targetFile", out.to_str().unwrap(), "-lang", "de", "-engine", "pseudo"], None);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("identity.md -> de"), "progress goes to stderr: {}", stderr(&o));
    assert!(std::fs::read_to_string(&out).unwrap().contains("ÜGETTING ÜSTARTED"));

    let quiet = run(&["-sourceFile", src.to_str().unwrap(), "-targetFile", out.to_str().unwrap(), "-lang", "de", "-engine", "pseudo", "-quiet"], None);
    assert!(quiet.status.success());
    assert_eq!((stdout(&quiet), stderr(&quiet)), (String::new(), String::new()));
}

#[test]
fn stdin_to_stdout_only_when_asked() {
    let o = run(&["-sourceFile", "-", "-targetFile", "-", "-lang", "fr", "-engine", "pseudo", "-quiet"], Some("# Hello\n\nSome *text*.\n"));
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "# <a id=\"hello\"></a>ÜHELLO\n\nÜSOME *ÜTEXT*.\n");
    assert_eq!(stderr(&o), "");
}

#[test]
fn a_target_file_is_required() {
    let src = fixture();
    let o = run(&["-sourceFile", src.to_str().unwrap(), "-lang", "de", "-engine", "pseudo"], None);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("-targetFile is required"), "{}", stderr(&o));
    let o = run(&["-targetFile", "-", "-lang", "de", "-engine", "pseudo"], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("-sourceFile is required"), "{}", stderr(&o));
}

#[test]
fn errors_go_to_stderr() {
    for args in [
        vec!["-bogus"],
        vec!["-sourceFile", "does-not-exist.md", "-targetFile", "-", "-lang", "de"],
        vec!["-sourceFile", "-", "-targetFile", "-", "-lang", "xx", "-engine", "pseudo"],
        vec!["-sourceFile", "-", "-targetFile", "-", "-lang", "de", "-reportFile", "-"],
        vec!["-sourceFile", "-", "-targetFile", "-", "-lang", "de"],
    ] {
        let o = run(&args, Some("text"));
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert_eq!(stdout(&o), "", "{args:?}");
        assert!(stderr(&o).starts_with("mdtranslate: "), "{args:?}: {}", stderr(&o));
    }
}

#[test]
fn version_on_stderr_help_on_stdout() {
    let v = run(&["-version"], None);
    assert_eq!(stdout(&v), "");
    assert!(stderr(&v).starts_with("mdtranslate "));
    let h = run(&["-help"], None);
    assert!(stdout(&h).contains("-sourceFile"));
    let h = run(&["-h"], None);
    assert!(stdout(&h).contains("-targetFile"));
}

#[test]
fn inspection_output_goes_to_the_target_file() {
    let o = run(&["-listLanguages"], None);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stdout(&o), "");
    let o = run(&["-listLanguages", "-targetFile", "-"], None);
    assert!(stdout(&o).contains("\"defaultTargets\""));

    let dump = tmp("extraction.json");
    let src = fixture();
    let o = run(&["-sourceFile", src.to_str().unwrap(), "-targetFile", dump.to_str().unwrap(), "-dumpExtraction"], None);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&dump).unwrap()).unwrap();
    assert!(v["segments"].as_array().is_some_and(|s| !s.is_empty()));
}

#[test]
fn several_languages_need_a_pattern() {
    let o = run(&["-sourceFile", "-", "-targetFile", "-", "-lang", "de,fr", "-engine", "pseudo"], Some("text"));
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(stdout(&o), "");
    let pattern = tmp("multi.{lang}.md");
    let o = run(&["-sourceFile", "-", "-targetFile", pattern.to_str().unwrap(), "-lang", "de,fr", "-engine", "pseudo", "-quiet"], Some("Some text.\n"));
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    for l in ["de", "fr"] {
        assert!(std::fs::read_to_string(pattern.to_str().unwrap().replace("{lang}", l)).unwrap().contains("ÜSOME"));
    }
}

fn run_env(args: &[&str], stdin: Option<&str>, env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.args(args).env_remove("AZURE_OPENAI_ENDPOINT").env_remove("AZURE_TRANSLATOR_ENDPOINT").env_remove("AZURE_AI_API_KEY");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    if let Some(input) = stdin {
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().unwrap()
}

#[test]
fn unreachable_service_fails_with_exit_code_2() {
    // Port 9 (discard) refuses connections; one attempt per request keeps the test fast.
    let env = [("AZURE_OPENAI_ENDPOINT", "http://127.0.0.1:9"), ("AZURE_AI_API_KEY", "test"), ("MDT_MAX_ATTEMPTS", "1")];
    let out = tmp("unreachable.de.md");
    let src = "# Title\n\nFirst paragraph.\n";
    let o = run_env(&["-sourceFile", "-", "-targetFile", out.to_str().unwrap(), "-lang", "de", "-quiet"], Some(src), &env);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("no segment could be translated"), "{}", stderr(&o));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), src, "the source is written unchanged");

    let o = run_env(&["-sourceFile", "-", "-targetFile", "-", "-analyzeOnly"], Some(src), &env);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("document analysis failed"), "{}", stderr(&o));

    let o = run_env(&["-sourceFile", "-", "-targetFile", "-", "-analyzeOnly"], Some(src), &[]);
    assert_eq!(o.status.code(), Some(1), "no endpoint configured");
}

#[test]
fn partial_translation_exits_with_3() {
    let tm = tmp("partial-tm.json");
    std::fs::write(&tm, r#"{"tm": [["s2", "Erster Absatz."]], "outcomes": [{"id": "s1", "kind": "heading", "via": "source", "retries": 2, "errors": ["model call failed: test"]}]}"#).unwrap();
    let o = run(&["-sourceFile", "-", "-targetFile", "-", "-lang", "de", "-tmFile", tm.to_str().unwrap(), "-quiet", "-no-preserveAnchors"], Some("# Title\n\nFirst paragraph.\n"));
    assert_eq!(o.status.code(), Some(3), "{}", stderr(&o));
    assert_eq!(stdout(&o), "# Title\n\nErster Absatz.\n");
    assert!(stderr(&o).contains("1 of 2 segments kept in the source language: model call failed: test"), "{}", stderr(&o));
}

/// A stand-in for Microsoft Entra ID and Azure OpenAI on a local port. It records each request path and whether it
/// carried the stub's bearer token (never the header itself, which could be a real token).
fn azure_stub() -> (String, std::sync::mpsc::Receiver<String>) {
    use std::io::{BufRead, BufReader, Read};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let path = request.split(' ').nth(1).unwrap_or("").split('?').next().unwrap_or("").to_string();
            let (mut len, mut stub_token) = (0, false);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line.trim().is_empty() {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap_or((&line, ""));
                match name.trim().to_ascii_lowercase().as_str() {
                    "content-length" => len = value.trim().parse().unwrap_or(0),
                    "authorization" => stub_token = value.trim() == "Bearer stub-token",
                    _ => {}
                }
            }
            reader.read_exact(&mut vec![0; len]).unwrap();
            let body = if path.ends_with("/oauth2/v2.0/token") {
                r#"{"token_type":"Bearer","access_token":"stub-token","expires_in":3600}"#.to_string()
            } else {
                let analysis = r#"{"sourceLanguage":"en","register":"neutral","registerEvidence":"Plain.","domain":"software","audience":"developers","summary":"A test.","doNotTranslate":[],"terminology":[]}"#;
                serde_json::json!({ "choices": [{ "message": { "content": analysis }, "finish_reason": "stop" }], "usage": { "prompt_tokens": 1, "completion_tokens": 1 } }).to_string()
            };
            let _ = tx.send(format!("{path} stub-token={stub_token}"));
            let _ = write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
        }
    });
    (base, rx)
}

#[test]
fn env_file_reaches_the_credential_chain() {
    let (base, requests) = azure_stub();
    let env_file = tmp("service-principal.env");
    std::fs::write(
        &env_file,
        format!("AZURE_OPENAI_ENDPOINT={base}\nAZURE_TENANT_ID=contoso.onmicrosoft.com\nAZURE_CLIENT_ID=11111111-2222-3333-4444-555555555555\nAZURE_CLIENT_SECRET=not-a-secret\nAZURE_AUTHORITY_HOST={base}\n"),
    )
    .unwrap();
    let mut cmd = Command::new(BIN);
    cmd.args(["-sourceFile", "-", "-targetFile", "-", "-analyzeOnly", "-envFile", env_file.to_str().unwrap()]);
    for (k, _) in std::env::vars_os() {
        if ["AZURE_", "IDENTITY_", "MSI_", "MDT_"].iter().any(|p| k.to_string_lossy().to_ascii_uppercase().starts_with(p)) {
            cmd.env_remove(k);
        }
    }
    // The managed identity (tried first, as AZURE_CLIENT_ID is set) is unreachable; the service principal from the
    // file has to answer.
    cmd.env("AZURE_POD_IDENTITY_AUTHORITY_HOST", "http://127.0.0.1:9");
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"# Title\n\nFirst paragraph.\n").unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(stdout(&o).contains("\"register\": \"neutral\""), "{}", stdout(&o));
    let seen: Vec<String> = requests.try_iter().collect();
    assert!(seen.contains(&"/contoso.onmicrosoft.com/oauth2/v2.0/token stub-token=false".to_string()), "{seen:?}");
    assert!(seen.contains(&"/openai/v1/chat/completions stub-token=true".to_string()), "{seen:?}");
}
