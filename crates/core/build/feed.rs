//! route.bible adapters feed ingest, shared by `build.rs` and the `sync_adapters_feed` test.
//!
//! Reads `{base}/latest.txt` and `manifest.json`, checks every icon against its sha256, and stages
//! `adapters.links.json`, `adapter-icons.json` and the icons. `base` is a URL or a local directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "../src/open_in_exclude.rs"]
mod exclude;
use exclude::NOT_IN_BIBLE_RS;

fn excluded(id: &str) -> bool {
    NOT_IN_BIBLE_RS.contains(&id)
}

#[derive(Debug)]
pub struct FeedError(pub String);
impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for FeedError {}

/// Fetches the feed at `base` into `out_dir/feed-cache/<version>/`, then stages its links at
/// `staged_links` and its icons in `icons_out`. Returns the feed version.
pub fn refresh(
    base: &str,
    out_dir: &Path,
    staged_links: &Path,
    icons_out: &Path,
) -> Result<String, FeedError> {
    let base = base.trim_end_matches('/').to_string();

    let latest_bytes = fetch(&base, "latest.txt")?;
    let version = String::from_utf8_lossy(&latest_bytes).trim().to_string();
    if !is_safe_name(&version) {
        return Err(FeedError(format!("bad latest.txt from {base}")));
    }

    let cache = out_dir.join("feed-cache").join(&version);
    fs::create_dir_all(&cache).map_err(|e| FeedError(e.to_string()))?;
    let manifest_path = cache.join("manifest.json");
    if !manifest_path.is_file() {
        let body = fetch(&base, "manifest.json")?;
        fs::write(&manifest_path, body).map_err(|e| FeedError(e.to_string()))?;
    }

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| FeedError(e.to_string()))?)
            .map_err(|e| FeedError(format!("manifest JSON: {e}")))?;

    validate_manifest(&manifest, &version)?;

    let links_cache = cache.join("adapters.links.json");
    let icons_cache = cache.join("icons");
    // Staging copies out of the cache; never let a destination be (or sit inside) the cache.
    if staged_links.starts_with(&cache) || icons_out.starts_with(&cache) || cache.starts_with(icons_out) {
        return Err(FeedError("staging paths overlap the feed cache".into()));
    }
    if !links_cache.is_file() {
        let adapters = manifest_adapters(&manifest)
            .ok_or_else(|| FeedError("manifest.adapters must be an array".into()))?;
        // One adapter per line keeps the vendored copy small and its diffs readable.
        let generated = manifest.get("generatedAt").cloned().unwrap_or_default();
        let lines: Vec<String> = adapters
            .iter()
            .filter(|a| !a.get("id").and_then(|v| v.as_str()).is_some_and(excluded))
            .map(|a| format!("  {a}"))
            .collect();
        let body = format!(
            "{{\"version\": {}, \"generatedAt\": {generated}, \"adapters\": [\n{}\n]}}\n",
            serde_json::Value::from(version.as_str()),
            lines.join(",\n")
        );
        fs::write(&links_cache, body).map_err(|e| FeedError(e.to_string()))?;
    }

    fs::create_dir_all(&icons_cache).map_err(|e| FeedError(e.to_string()))?;

    let icons = manifest
        .get("icons")
        .and_then(|v| v.as_object())
        .ok_or_else(|| FeedError("manifest.icons must be an object".into()))?;

    let icons: Vec<_> = icons.iter().filter(|(id, _)| !excluded(id)).collect();
    let mut icon_records = Vec::new();
    for &(id, meta) in &icons {
        let path = meta
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError(format!("icons[{id}].path")))?;
        let expect = meta
            .get("sha256")
            .and_then(|v| v.as_str())
            .map(|s| s.to_ascii_lowercase())
            .ok_or_else(|| FeedError(format!("icons[{id}].sha256")))?;
        let dest = icons_cache.join(format!("{id}.png"));
        if !dest.is_file() {
            let body = fetch(&base, feed_relative(path))?;
            let got = sha256_hex(&body);
            if got != expect {
                return Err(FeedError(format!("icon {id} sha256 mismatch: got {got}, expected {expect}")));
            }
            fs::write(&dest, &body).map_err(|e| FeedError(e.to_string()))?;
        } else {
            let body = fs::read(&dest).map_err(|e| FeedError(e.to_string()))?;
            let got = sha256_hex(&body);
            if got != expect {
                return Err(FeedError(format!("cached icon {id} sha256 mismatch")));
            }
        }
        // Only route.bible's display hints are kept: `fit` (cover = rounded tile, contain = bare
        // logo) and `theme`. The PNG itself is verified above.
        let mut record = serde_json::json!({ "id": id });
        for hint in ["fit", "theme"] {
            if let Some(v) = meta.get(hint) {
                record[hint] = v.clone();
            }
        }
        icon_records.push(record);
    }

    icon_records.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    // One icon per line.
    let lines: Vec<String> = icon_records.iter().map(|r| format!("  {r}")).collect();
    let icons_json = format!(
        "{{\"version\": 1, \"source\": \"route.bible adapters feed ({version})\", \"icons\": [\n{}\n]}}\n",
        lines.join(",\n")
    );
    fs::write(cache.join("adapter-icons.json"), icons_json).map_err(|e| FeedError(e.to_string()))?;
    // Everything verified: only now replace the staged snapshot.
    let _ = fs::remove_dir_all(icons_out);
    fs::create_dir_all(icons_out).map_err(|e| FeedError(e.to_string()))?;
    for (id, _) in &icons {
        fs::copy(icons_cache.join(format!("{id}.png")), icons_out.join(format!("{id}.png")))
            .map_err(|e| FeedError(e.to_string()))?;
    }
    fs::copy(&links_cache, staged_links).map_err(|e| FeedError(e.to_string()))?;
    fs::write(cache.join("adapters.version"), format!("{version}\n"))
        .map_err(|e| FeedError(e.to_string()))?;

    Ok(version)
}

/// The feed's adapter list: `adapters: [...]` (live feed) or the older `adapters: { adapters: [...] }`.
fn manifest_adapters(manifest: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    let adapters = manifest.get("adapters")?;
    adapters.as_array().or_else(|| adapters.get("adapters").and_then(|v| v.as_array()))
}

fn validate_manifest(manifest: &serde_json::Value, latest: &str) -> Result<(), FeedError> {
    let schema = manifest.get("schemaVersion").and_then(|v| v.as_u64());
    if schema != Some(1) {
        return Err(FeedError(format!("manifest schemaVersion={schema:?}, expected 1")));
    }
    for key in ["version", "generatedAt", "adapterCount", "adapters", "icons"] {
        if manifest.get(key).is_none() {
            return Err(FeedError(format!("manifest.json missing {key}")));
        }
    }
    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| FeedError("manifest.version must be a string".into()))?
        .trim();
    if version != latest {
        return Err(FeedError(format!("manifest.version={version:?} != latest.txt={latest:?}")));
    }
    let adapters =
        manifest_adapters(manifest).ok_or_else(|| FeedError("manifest.adapters must be an array".into()))?;
    for (i, adapter) in adapters.iter().enumerate() {
        validate_adapter(adapter).map_err(|field| FeedError(format!("manifest.adapters[{i}].{field}")))?;
    }
    let count = manifest
        .get("adapterCount")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FeedError("manifest.adapterCount must be an integer".into()))?;
    if count as usize != adapters.len() {
        return Err(FeedError(format!("manifest.adapterCount={count} != len(adapters)={}", adapters.len())));
    }
    let icons = manifest
        .get("icons")
        .and_then(|v| v.as_object())
        .ok_or_else(|| FeedError("manifest.icons must be an object".into()))?;
    for (id, meta) in icons {
        if !is_safe_name(id) {
            return Err(FeedError(format!("icons[{id:?}] is not a plain file name")));
        }
        let sha = meta
            .get("sha256")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError(format!("icons[{id}].sha256")))?;
        if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(FeedError(format!("icons[{id}].sha256 is not 64-char hex")));
        }
        let path = meta
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError(format!("icons[{id}].path")))?;
        if path.split(['/', '\\']).any(|part| part == "..") {
            return Err(FeedError(format!("icons[{id}].path leaves the feed")));
        }
    }
    Ok(())
}

/// Versions and icon ids become file names under `OUT_DIR`: ASCII letters, digits, `.`, `-`, `_`,
/// not starting with `.`, so no separators, `..` or absolute paths.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Checks one adapter against what `open_in::Adapter` / `LinkTemplate` deserialize at startup
/// (`Catalog::bundled` panics on a bad entry). Returns the first bad field.
fn validate_adapter(a: &serde_json::Value) -> Result<(), &'static str> {
    use serde_json::Value;
    let string = |v: Option<&Value>| v.is_some_and(Value::is_string);
    let defaulted_string = |v: Option<&Value>| v.is_none_or(Value::is_string);
    let optional_string = |v: Option<&Value>| v.is_none_or(|v| v.is_null() || v.is_string());
    if !a.is_object() {
        return Err("(not an object)");
    }
    if !string(a.get("id")) {
        return Err("id");
    }
    if !string(a.get("name")) {
        return Err("name");
    }
    if !a.get("featuredRank").is_none_or(|v| v.is_null() || v.as_u64().is_some_and(|n| n <= u32::MAX as u64))
    {
        return Err("featuredRank");
    }
    if !defaulted_string(a.get("status")) {
        return Err("status");
    }
    if !optional_string(a.get("catalogType")) {
        return Err("catalogType");
    }
    if !optional_string(a.get("catalogSubtype")) {
        return Err("catalogSubtype");
    }
    let Some(links) = a.get("links") else { return Ok(()) };
    let links = links.as_array().ok_or("links")?;
    for link in links {
        if !link.is_object() || !string(link.get("kind")) || !string(link.get("template")) {
            return Err("links");
        }
        let platforms_ok = link
            .get("platforms")
            .is_none_or(|p| p.as_array().is_some_and(|p| p.iter().all(Value::is_string)));
        if !platforms_ok {
            return Err("links.platforms");
        }
    }
    Ok(())
}

/// Icon paths come feed-relative (`icons/x.png`) or site-absolute (`/adapters/v1/icons/x.png`).
fn feed_relative(path: &str) -> &str {
    path.rsplit_once("/adapters/v1/").map(|(_, rest)| rest).unwrap_or(path)
}

fn fetch(base: &str, rel: &str) -> Result<Vec<u8>, FeedError> {
    let rel = rel.trim_start_matches('/');
    let local = Path::new(base);
    if local.is_dir() || base.starts_with("file:") {
        let root = PathBuf::from(base.strip_prefix("file://").unwrap_or(base));
        let path = root.join(rel);
        let body = fs::read(&path).map_err(|e| FeedError(format!("read {}: {e}", path.display())))?;
        return Ok(body);
    }
    let url = format!("{base}/{rel}");
    http_get(&url)
}

fn http_get(url: &str) -> Result<Vec<u8>, FeedError> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--retry",
            "2",
            "--retry-delay",
            "1",
            "-A",
            "bible-rs-build/0.2 (adapters refresh)",
            url,
        ])
        .output()
        .map_err(|e| FeedError(format!("curl {url}: {e}")))?;
    if !output.status.success() {
        return Err(FeedError(format!("curl {url} exited {}", output.status.code().unwrap_or(-1))));
    }
    if output.stdout.starts_with(b"<!doctype") || output.stdout.starts_with(b"<!DOCTYPE") {
        return Err(FeedError(format!("{url} returned HTML, not the feed")));
    }
    Ok(output.stdout)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    sha256::digest(bytes)
}

mod sha256 {
    pub fn digest(data: &[u8]) -> String {
        let mut h: [u32; 8] =
            [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
        let bit_len = (data.len() as u64) * 8;
        let mut buf = data.to_vec();
        buf.push(0x80);
        while (buf.len() % 64) != 56 {
            buf.push(0);
        }
        buf.extend_from_slice(&bit_len.to_be_bytes());
        for chunk in buf.as_chunks::<64>().0 {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }
            let mut a = h;
            for i in 0..64 {
                let s1 = a[4].rotate_right(6) ^ a[4].rotate_right(11) ^ a[4].rotate_right(25);
                let ch = (a[4] & a[5]) ^ ((!a[4]) & a[6]);
                let t1 = a[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
                let s0 = a[0].rotate_right(2) ^ a[0].rotate_right(13) ^ a[0].rotate_right(22);
                let maj = (a[0] & a[1]) ^ (a[0] & a[2]) ^ (a[1] & a[2]);
                let t2 = s0.wrapping_add(maj);
                a[7] = a[6];
                a[6] = a[5];
                a[5] = a[4];
                a[4] = a[3].wrapping_add(t1);
                a[3] = a[2];
                a[2] = a[1];
                a[1] = a[0];
                a[0] = t1.wrapping_add(t2);
            }
            for i in 0..8 {
                h[i] = h[i].wrapping_add(a[i]);
            }
        }
        h.iter().map(|x| format!("{x:08x}")).collect()
    }

    #[rustfmt::skip]
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
}
