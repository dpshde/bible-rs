//! The route.bible feed ingest (`build/feed.rs`, used by `build.rs` and `scripts/sync-adapters`)
//! against a tiny synthetic feed built here from two vendored icons: sha256 checks, the manifest
//! fields it tolerates, and failures that make the build keep the vendored snapshot.

#![allow(dead_code)]

#[path = "../build/feed.rs"]
mod feed;

use std::fs;
use std::path::{Path, PathBuf};

const VERSION: &str = "2026.01.01-test";

fn icon(id: &str) -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("data/adapter-icons/{id}.png"))).unwrap()
}

/// A two-adapter feed in a fresh directory, shaped like route.bible's live `adapters/v1`.
fn synthetic_feed(name: &str, tamper: impl Fn(&mut serde_json::Value)) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("icons")).unwrap();
    let mut icons = serde_json::Map::new();
    for (id, fit) in [("bible-app", "cover"), ("bible-is", "contain")] {
        let png = icon(id);
        fs::write(dir.join(format!("icons/{id}.png")), &png).unwrap();
        icons.insert(
            id.into(),
            serde_json::json!({
                "path": format!("/adapters/v1/icons/{id}.png"),
                "sha256": feed::sha256_hex(&png),
                "width": 64, "height": 64, "fit": fit,
            }),
        );
    }
    let link = |url: &str| serde_json::json!([{ "kind": "web", "template": url, "platforms": ["web"] }]);
    let mut manifest = serde_json::json!({
        "schemaVersion": 1,
        "version": VERSION,
        "generatedAt": "2026-01-01T00:00:00.000Z",
        "adapterCount": 2,
        // Newer feeds document their template variables; ingest ignores them.
        "templateVariableNames": ["passage", "verseText"],
        "templateVariableNotes": { "verseText": { "maxLength": 500, "fallback": "displayPassage" } },
        "adapters": [
            { "id": "bible-app", "name": "YouVersion", "links": link("https://www.bible.com/bible/{{youversionVersionId}}/{{passage}}") },
            { "id": "bible-is", "name": "Bible.is", "links": link("https://live.bible.is/bible/{{bibleIsPath}}") },
        ],
        "icons": icons,
    });
    tamper(&mut manifest);
    fs::write(dir.join("latest.txt"), format!("{VERSION}\n")).unwrap();
    fs::write(dir.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
    dir
}

fn ingest(feed_dir: &Path) -> (Result<String, feed::FeedError>, PathBuf) {
    let out = feed_dir.with_extension("out");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    let r =
        feed::refresh(feed_dir.to_str().unwrap(), &out, &out.join("adapters.links.json"), &out.join("icons"));
    (r, out)
}

#[test]
fn ingests_a_feed_and_keeps_display_hints() {
    let (r, out) = ingest(&synthetic_feed("feed-ok", |_| {}));
    assert_eq!(r.unwrap(), VERSION);
    let links: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("adapters.links.json")).unwrap()).unwrap();
    assert_eq!(links["version"], VERSION);
    assert_eq!(links["adapters"].as_array().unwrap().len(), 2);
    assert_eq!(fs::read(out.join("icons/bible-is.png")).unwrap(), icon("bible-is"));
    let meta: serde_json::Value = serde_json::from_slice(
        &fs::read(out.join("feed-cache").join(VERSION).join("adapter-icons.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(meta["icons"][1], serde_json::json!({ "id": "bible-is", "fit": "contain" }));
}

#[test]
fn rejects_an_icon_whose_sha256_does_not_match() {
    let (r, out) = ingest(&synthetic_feed("feed-bad-sha", |m| {
        m["icons"]["bible-is"]["sha256"] = "0".repeat(64).into();
    }));
    let err = r.unwrap_err().0;
    assert!(err.contains("bible-is sha256 mismatch"), "{err}");
    // build.rs then warns and keeps the vendored snapshot; nothing was staged.
    assert!(!out.join("adapters.links.json").exists());
}

#[test]
fn rejects_inconsistent_or_missing_feeds() {
    let (r, _) = ingest(&synthetic_feed("feed-bad-count", |m| m["adapterCount"] = 3.into()));
    assert!(r.unwrap_err().0.contains("adapterCount"));
    let (r, _) = ingest(&synthetic_feed("feed-bad-version", |m| m["version"] = "other".into()));
    assert!(r.unwrap_err().0.contains("latest.txt"));
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("feed-missing");
    let _ = fs::remove_dir_all(&missing);
    fs::create_dir_all(&missing).unwrap();
    let (r, _) = ingest(&missing);
    assert!(r.unwrap_err().0.contains("latest.txt"));
}
