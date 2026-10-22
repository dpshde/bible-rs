//! Open a passage in another Bible app or site via [route.bible](https://route.bible) adapters.
//!
//! The adapter list and link templates are **static**: vendored from route.bible's
//! `adapters.links.json` and embedded at compile time. Choosing an entry expands a local
//! template for the current verse/range (no network). Specialty templates we cannot fill fall
//! back to a route.bible launcher URL the browser finishes. Refresh the vendored file with
//! `scripts/sync-adapters`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::books::VerseRef;

const API_BASE: &str = "https://route.bible";
const BUNDLED_LINKS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/adapters.links.json"));

/// A same-chapter passage: one verse, or an inclusive range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Passage {
    pub book: crate::books::BookId,
    pub chapter: u8,
    pub verse_start: u8,
    pub verse_end: u8,
}

impl Passage {
    /// From a non-empty selection of same-chapter verses (order does not matter).
    pub fn from_refs(refs: &[VerseRef]) -> Option<Self> {
        let first = *refs.first()?;
        if refs.iter().any(|r| r.book != first.book || r.chapter != first.chapter) {
            return None;
        }
        let start = refs.iter().map(|r| r.verse).min()?;
        let end = refs.iter().map(|r| r.verse).max()?;
        Some(Self { book: first.book, chapter: first.chapter, verse_start: start, verse_end: end })
    }

    pub fn is_range(self) -> bool {
        self.verse_end > self.verse_start
    }

    /// "Romans 5:3" or "Romans 5:3–5".
    pub fn display(self) -> String {
        let name = self.book.ref_name();
        if self.is_range() {
            format!("{name} {}:{}–{}", self.chapter, self.verse_start, self.verse_end)
        } else {
            format!("{name} {}:{}", self.chapter, self.verse_start)
        }
    }

    /// ASCII display used in many query strings: "Romans 5:3-5".
    pub fn display_ascii(self) -> String {
        self.display().replace('–', "-")
    }

    /// Uppercase OSIS-style canonical: `ROM.5.3` or `ROM.5.3-5`.
    pub fn canonical(self) -> String {
        let code = self.book.info().code;
        if self.is_range() {
            format!("{code}.{}.{start}-{end}", self.chapter, start = self.verse_start, end = self.verse_end)
        } else {
            format!("{code}.{}.{}", self.chapter, self.verse_start)
        }
    }

    /// Lowercase route path: `/rom.5.3-5`.
    pub fn route_path(self) -> String {
        format!("/{}", self.canonical().to_ascii_lowercase())
    }

    /// Values for the common `{{…}}` placeholders we expand locally.
    pub fn template_vars(self, translation: &str) -> BTreeMap<&'static str, String> {
        let display = self.display_ascii();
        let canonical = self.canonical();
        let path = self.route_path();
        let mut m = BTreeMap::new();
        m.insert("displayPassage", display.clone());
        m.insert("passage", canonical.clone());
        m.insert("passageLower", canonical.to_ascii_lowercase());
        m.insert("bookCode", self.book.info().code.to_ascii_lowercase());
        m.insert("chapter", self.chapter.to_string());
        m.insert("verse", self.verse_start.to_string());
        m.insert(
            "verseRange",
            if self.is_range() {
                format!("{}-{}", self.verse_start, self.verse_end)
            } else {
                self.verse_start.to_string()
            },
        );
        m.insert("translation", translation.to_string());
        m.insert("translationLower", translation.to_ascii_lowercase());
        m.insert("routeUrl", format!("{API_BASE}{path}"));
        m.insert("shareText", display);
        m
    }

    /// Launcher URL that route.bible finishes when a local template cannot be expanded.
    pub fn fallback_open_url(self, adapter_id: &str) -> String {
        format!("{API_BASE}{}?app={adapter_id}&mode=web", self.route_path())
    }
}

/// One destination in the Open in › submenu (from the vendored links snapshot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adapter {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub featured_rank: Option<u32>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub catalog_type: Option<String>,
    #[serde(default)]
    pub catalog_subtype: Option<String>,
    #[serde(default)]
    pub links: Vec<LinkTemplate>,
}

/// A link template from the vendored `@route-bible/adapters` links snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTemplate {
    pub kind: String,
    pub template: String,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LinksFile {
    adapters: Vec<Adapter>,
}

fn bundled_links() -> LinksFile {
    serde_json::from_str(BUNDLED_LINKS).expect("bundled adapters.links.json")
}

/// Static adapter catalog for the Open in › submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub items: Vec<Adapter>,
}

impl Catalog {
    /// Embedded snapshot — no disk I/O, no network.
    pub fn bundled() -> Self {
        let file = bundled_links();
        Self { items: file.adapters }
    }

    /// Featured (by rank) first, then the rest A–Z by name. Skips disabled entries.
    pub fn menu_items(&self) -> Vec<&Adapter> {
        let mut featured: Vec<&Adapter> = Vec::new();
        let mut rest: Vec<&Adapter> = Vec::new();
        for a in &self.items {
            if a.status == "disabled" {
                continue;
            }
            if a.featured_rank.is_some() {
                featured.push(a);
            } else {
                rest.push(a);
            }
        }
        featured.sort_by_key(|a| a.featured_rank.unwrap_or(u32::MAX));
        rest.sort_by_key(|a| a.name.to_ascii_lowercase());
        featured.extend(rest);
        featured
    }
}

/// Expand `{{placeholders}}` when every one is in `vars`. Returns `None` if any are unknown
/// (specialty adapters fall back to the route.bible launcher URL).
pub fn expand_template(template: &str, vars: &BTreeMap<&str, String>) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}")?;
        let key = after[..end].trim();
        let value = vars.get(key)?;
        // Query-ish templates expect URL encoding for free text.
        if key == "displayPassage" || key == "shareText" || key == "routeUrl" {
            out.push_str(&urlencoding_minimal(value));
        } else {
            out.push_str(value);
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Some(out)
}

fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            b':' => out.push_str("%3A"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Prefer a web template, else the first deep link, when local expansion succeeds.
pub fn expand_adapter_url(adapter_id: &str, passage: Passage, translation: &str) -> Option<String> {
    let catalog = Catalog::bundled();
    let adapter = catalog.items.iter().find(|a| a.id == adapter_id)?;
    let vars = passage.template_vars(translation);
    let web = adapter.links.iter().find(|l| l.kind == "web");
    let deep = adapter.links.iter().find(|l| l.kind == "deep");
    for link in [web, deep].into_iter().flatten() {
        if let Some(url) = expand_template(&link.template, &vars) {
            // Relative templates are route.bible paths.
            if url.starts_with('/') {
                return Some(format!("{API_BASE}{url}"));
            }
            return Some(url);
        }
    }
    None
}

/// Best URL for opening `adapter_id` with `passage`: local template expansion, else the
/// route.bible launcher fallback. Never contacts the network from this process.
pub fn open_url_for(passage: Passage, adapter_id: &str, translation: &str) -> String {
    expand_adapter_url(adapter_id, passage, translation)
        .unwrap_or_else(|| passage.fallback_open_url(adapter_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::BookId;

    fn john(n: u8) -> VerseRef {
        VerseRef::new(BookId(42), 3, n)
    }
    fn rom(n: u8) -> VerseRef {
        VerseRef::new(BookId(44), 5, n)
    }

    #[test]
    fn passage_from_single_and_range() {
        let one = Passage::from_refs(&[john(16)]).unwrap();
        assert_eq!(one.display(), "John 3:16");
        assert_eq!(one.canonical(), "JHN.3.16");
        assert_eq!(one.route_path(), "/jhn.3.16");
        assert!(!one.is_range());

        let range = Passage::from_refs(&[rom(5), rom(3), rom(4)]).unwrap();
        assert_eq!(range.display(), "Romans 5:3–5");
        assert_eq!(range.display_ascii(), "Romans 5:3-5");
        assert_eq!(range.canonical(), "ROM.5.3-5");
        assert_eq!(range.route_path(), "/rom.5.3-5");
        assert!(range.is_range());
    }

    #[test]
    fn passage_rejects_cross_chapter() {
        let a = VerseRef::new(BookId(42), 3, 16);
        let b = VerseRef::new(BookId(42), 4, 1);
        assert!(Passage::from_refs(&[a, b]).is_none());
    }

    #[test]
    fn expand_template_fills_known_placeholders() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let vars = p.template_vars("ESV");
        let url = expand_template(
            "https://www.biblegateway.com/passage/?search={{displayPassage}}&version={{translation}}",
            &vars,
        )
        .unwrap();
        assert_eq!(url, "https://www.biblegateway.com/passage/?search=John%203%3A16&version=ESV");

        let range = Passage::from_refs(&[rom(3), rom(5)]).unwrap();
        let vars = range.template_vars("BSB");
        let url = expand_template(
            "https://www.biblegateway.com/passage/?search={{displayPassage}}&version={{translation}}",
            &vars,
        )
        .unwrap();
        assert!(url.contains("Romans%205%3A3-5"));
        assert!(url.ends_with("version=BSB"));
    }

    #[test]
    fn expand_template_rejects_unknown_placeholders() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let vars = p.template_vars("ESV");
        assert!(expand_template("https://x.test/{{youversionVersionId}}/{{passage}}", &vars).is_none());
    }

    #[test]
    fn bundled_catalog_lists_featured_first() {
        let c = Catalog::bundled();
        assert!(c.items.len() >= 50, "bundled links should ship the public adapters");
        let menu = c.menu_items();
        assert_eq!(menu[0].id, "bible-app");
        assert_eq!(menu[0].featured_rank, Some(1));
        assert_eq!(menu[1].id, "bible-gateway");
        let first_plain = menu.iter().position(|a| a.featured_rank.is_none()).unwrap();
        assert!(
            menu[first_plain].name.to_ascii_lowercase() <= menu[first_plain + 1].name.to_ascii_lowercase()
        );
    }

    #[test]
    fn simple_adapter_expands_from_bundled_links() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let url = expand_adapter_url("bible-gateway", p, "ESV").expect("bible-gateway template");
        assert!(url.starts_with("https://www.biblegateway.com/passage/?search="));
        assert!(url.contains("John%203%3A16"));
    }

    #[test]
    fn specialty_adapter_falls_back_to_route_bible() {
        let p = Passage::from_refs(&[rom(3), rom(5)]).unwrap();
        // Logos needs logosRefLyReference — we do not invent it; open via the launcher URL.
        let url = open_url_for(p, "logos", "ESV");
        assert_eq!(url, "https://route.bible/rom.5.3-5?app=logos&mode=web");
    }

    #[test]
    fn fallback_open_url_points_at_route_bible() {
        let p = Passage::from_refs(&[rom(3), rom(5)]).unwrap();
        assert_eq!(p.fallback_open_url("logos"), "https://route.bible/rom.5.3-5?app=logos&mode=web");
    }

    #[test]
    fn open_url_for_never_needs_network_for_simple_adapters() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let url = open_url_for(p, "bible-gateway", "ESV");
        assert!(url.contains("biblegateway.com"));
        assert!(!url.contains("route.bible"));
    }
}
