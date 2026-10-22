//! Contract tests for the route.bible adapters feed fixture.
//!
//! Prod/preview feeds are not always live; the local fixture under
//! `tests/fixtures/route-bible-adapters-feed/` is the offline stand-in for
//! `ROUTE_BIBLE_ADAPTERS=refresh` / `scripts/sync-adapters`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/route-bible-adapters-feed")
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
    // The live feed lists adapters directly; the first preview nested them under `adapters`.
    let adapters = manifest["adapters"]
        .as_array()
        .or_else(|| manifest["adapters"]["adapters"].as_array())
        .expect("adapters array");
    assert_eq!(manifest["adapterCount"].as_u64().unwrap() as usize, adapters.len());
    let icons = manifest["icons"].as_object().expect("icons object");
    assert!(!icons.is_empty());
    for (id, meta) in icons {
        let path = meta["path"].as_str().expect("icon.path");
        let sha = meta["sha256"].as_str().expect("icon.sha256");
        assert_eq!(sha.len(), 64, "icon {id} sha256 length");
        // Icon paths are feed-relative (`icons/x.png`) or site-absolute (`/adapters/v1/icons/x.png`).
        let file = dir.join(path.rsplit_once("/adapters/v1/").map(|(_, rest)| rest).unwrap_or(path));
        assert!(file.is_file(), "missing icon file {path}");
        assert_eq!(sha256_file(&file), sha.to_ascii_lowercase(), "icon {id} sha256");
        assert_eq!(meta["width"], 64);
        assert_eq!(meta["height"], 64);
    }
}
