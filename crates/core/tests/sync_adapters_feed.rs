//! Contract tests for the route.bible adapters feed fixture and `scripts/sync-adapters`.
//!
//! Prod/preview feeds are not always live; the local fixture under
//! `tests/fixtures/route-bible-adapters-feed/` is the offline stand-in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/route-bible-adapters-feed")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/core -> repo root")
        .to_path_buf()
}

fn sha256_file(path: &Path) -> String {
    let output =
        Command::new("sha256sum").arg(path).output().unwrap_or_else(|e| panic!("sha256sum {path:?}: {e}"));
    assert!(output.status.success(), "sha256sum failed for {path:?}");
    let line = String::from_utf8_lossy(&output.stdout);
    line.split_whitespace().next().expect("sha256sum digest").to_ascii_lowercase()
}

#[test]
fn fixture_manifest_matches_schema_v1() {
    let dir = fixture_dir();
    let latest = fs::read_to_string(dir.join("latest.txt")).expect("latest.txt").trim().to_string();
    assert!(!latest.is_empty(), "latest.txt must be non-empty");

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("manifest.json")).expect("manifest.json"))
            .expect("manifest JSON");
    assert_eq!(manifest["schemaVersion"], 1);
    assert_eq!(manifest["version"].as_str().unwrap(), latest);
    assert!(manifest["generatedAt"].as_str().is_some());
    let adapters = manifest["adapters"]["adapters"].as_array().expect("adapters.adapters array");
    assert_eq!(manifest["adapterCount"].as_u64().unwrap() as usize, adapters.len());
    let icons = manifest["icons"].as_object().expect("icons object");
    assert!(!icons.is_empty());
    for (id, meta) in icons {
        let path = meta["path"].as_str().expect("icon.path");
        let sha = meta["sha256"].as_str().expect("icon.sha256");
        assert_eq!(sha.len(), 64, "icon {id} sha256 length");
        let file = dir.join(path);
        assert!(file.is_file(), "missing icon file {path}");
        assert_eq!(sha256_file(&file), sha.to_ascii_lowercase(), "icon {id} sha256");
        assert_eq!(meta["width"], 64);
        assert_eq!(meta["height"], 64);
    }
}

#[test]
fn sync_adapters_dry_run_against_fixture() {
    let root = repo_root();
    let script = root.join("scripts/sync-adapters");
    let feed = fixture_dir();
    // Empty --root so we do not short-circuit on the committed adapters.version.
    let tmp = tempfile_dir();
    let output = Command::new(&script)
        .env("ROUTE_BIBLE_FEED", &feed)
        .arg("--dry-run")
        .arg("--root")
        .arg(&tmp)
        .output()
        .unwrap_or_else(|e| panic!("run sync-adapters: {e}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "sync-adapters --dry-run failed:\n{stdout}\n{stderr}");
    assert!(stdout.contains("dry-run"), "expected dry-run confirmation, got:\n{stdout}");
    let _ = fs::remove_dir_all(&tmp);
}

fn tempfile_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bible-rs-sync-adapters-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    dir
}
