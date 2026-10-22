//! Open a passage in another Bible app or site via [route.bible](https://route.bible) adapters.
//!
//! The adapter list and link templates are **static**: vendored from route.bible's adapters feed
//! into `crates/core/data/` by `scripts/sync-adapters` and embedded at compile time. Choosing an
//! entry expands a local template for the current verse/range — no runtime network. Desktop only
//! uses links whose `platforms` include `web` (or a desktop OS); mobile-only deep links are never
//! opened. Specialty templates that still cannot expand fall back to a route.bible launcher URL.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::books::VerseRef;
use crate::open_in_context;

const API_BASE: &str = "https://route.bible";

/// Links JSON embedded at compile time. `build.rs` stages the committed `data/` snapshot into `OUT_DIR`.
fn bundled_links_json() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/adapters.links.json"))
}

/// A same-chapter passage: one verse, an inclusive range, or a whole chapter (`verse_start == 0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Passage {
    pub book: crate::books::BookId,
    pub chapter: u8,
    /// First verse, or `0` for a chapter-only reference.
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

    /// Whole chapter (no verse): `John 3` → `JHN.3`.
    pub fn chapter_only(book: crate::books::BookId, chapter: u8) -> Self {
        Self { book, chapter, verse_start: 0, verse_end: 0 }
    }

    pub fn has_verse(self) -> bool {
        self.verse_start > 0
    }

    pub fn is_range(self) -> bool {
        self.has_verse() && self.verse_end > self.verse_start
    }

    /// "Romans 5:3", "Romans 5:3–5", or "John 3".
    pub fn display(self) -> String {
        let name = self.book.ref_name();
        if !self.has_verse() {
            format!("{name} {}", self.chapter)
        } else if self.is_range() {
            format!("{name} {}:{}–{}", self.chapter, self.verse_start, self.verse_end)
        } else {
            format!("{name} {}:{}", self.chapter, self.verse_start)
        }
    }

    /// ASCII display used in many query strings: "Romans 5:3-5".
    pub fn display_ascii(self) -> String {
        self.display().replace('–', "-")
    }

    /// Uppercase OSIS-style canonical: `ROM.5.3`, `ROM.5.3-5`, or `JHN.3`.
    pub fn canonical(self) -> String {
        let code = self.book.info().code;
        if !self.has_verse() {
            format!("{code}.{}", self.chapter)
        } else if self.is_range() {
            format!("{code}.{}.{start}-{end}", self.chapter, start = self.verse_start, end = self.verse_end)
        } else {
            format!("{code}.{}.{}", self.chapter, self.verse_start)
        }
    }

    /// Lowercase route path: `/rom.5.3-5`.
    pub fn route_path(self) -> String {
        format!("/{}", self.canonical().to_ascii_lowercase())
    }

    /// Values for every `{{…}}` placeholder the vendored adapters use (pre-encoded where needed).
    pub fn template_vars(self, translation: &str) -> BTreeMap<&'static str, String> {
        open_in_context::template_replacements(self, translation)
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

impl LinkTemplate {
    /// Desktop Open in › only uses links that target the web (or an explicit desktop OS).
    pub fn usable_on_desktop(&self) -> bool {
        self.platforms.iter().any(|p| matches!(p.as_str(), "web" | "macos" | "windows" | "linux" | "desktop"))
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LinksFile {
    adapters: Vec<Adapter>,
}

fn bundled_links() -> LinksFile {
    serde_json::from_str(bundled_links_json()).expect("bundled adapters.links.json")
}

/// Static adapter catalog for the Open in › submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub items: Vec<Adapter>,
}

impl Catalog {
    /// Embedded snapshot — no disk I/O, no network at runtime.
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

/// Expand `{{placeholders}}` when every one is in `vars`. Returns `None` if any are unknown.
/// Values in `vars` are already prepared for substitution (encoded where route.bible encodes them).
pub fn expand_template(template: &str, vars: &BTreeMap<&str, String>) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}")?;
        let key = after[..end].trim();
        let value = vars.get(key)?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    if out.contains("{{") {
        return None;
    }
    Some(out)
}

fn finalize_desktop_url(url: String) -> Option<String> {
    let url = if url.starts_with('/') { format!("{API_BASE}{url}") } else { url };
    if url.starts_with("https://") || url.starts_with("http://") { Some(url) } else { None }
}

/// Expand a desktop-usable link for `adapter_id`. Skips ios/android-only deep links.
/// Returns `None` when no web/desktop template expands to an http(s) URL.
pub fn expand_adapter_url(adapter_id: &str, passage: Passage, translation: &str) -> Option<String> {
    let catalog = Catalog::bundled();
    let adapter = catalog.items.iter().find(|a| a.id == adapter_id)?;
    let vars = passage.template_vars(translation);
    let usable: Vec<&LinkTemplate> = adapter.links.iter().filter(|l| l.usable_on_desktop()).collect();
    // Prefer kind=web, then any other desktop-usable link.
    let ordered = usable
        .iter()
        .copied()
        .filter(|l| l.kind == "web")
        .chain(usable.iter().copied().filter(|l| l.kind != "web"));
    for link in ordered {
        // open-bible-geo can yield an empty path for unsupported books — treat as unusable.
        if link.template.contains("{{openBibleGeoPath}}")
            && vars.get("openBibleGeoPath").is_some_and(|v| v.is_empty())
        {
            continue;
        }
        if let Some(url) = expand_template(&link.template, &vars)
            && let Some(url) = finalize_desktop_url(url)
        {
            return Some(url);
        }
    }
    None
}

/// Whether [`open_url_for`] would use the route.bible launcher (no local web expansion).
pub fn uses_launcher_fallback(adapter_id: &str, passage: Passage, translation: &str) -> bool {
    expand_adapter_url(adapter_id, passage, translation).is_none()
}

/// Best URL for opening `adapter_id` with `passage` on desktop: local web template expansion,
/// else the route.bible launcher. Never returns a mobile-only deep link.
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
    fn passage_chapter_only() {
        let p = Passage::chapter_only(BookId(42), 3);
        assert_eq!(p.display(), "John 3");
        assert_eq!(p.canonical(), "JHN.3");
        assert!(!p.has_verse());
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
    }

    #[test]
    fn expand_template_rejects_unknown_placeholders() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let vars = p.template_vars("ESV");
        assert!(expand_template("https://x.test/{{notARealVar}}/{{passage}}", &vars).is_none());
    }

    #[test]
    fn bundled_catalog_lists_featured_first() {
        let c = Catalog::bundled();
        assert!(c.items.len() >= 50, "bundled links should ship the public adapters");
        let menu = c.menu_items();
        assert_eq!(menu[0].id, "bible-app");
        assert_eq!(menu[0].featured_rank, Some(1));
        assert_eq!(menu[1].id, "bible-gateway");
    }

    #[test]
    fn bible_app_opens_https_youversion_web_on_desktop() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let url = open_url_for(p, "bible-app", "ESV");
        assert!(
            url.starts_with("https://www.bible.com/bible/59/"),
            "expected YouVersion web URL for ESV, got {url}"
        );
        assert!(url.contains("JHN.3.16"));
        assert!(url.ends_with(".ESV"));
        assert!(!url.contains("youversion://"));
    }

    #[test]
    fn never_picks_ios_android_only_deep_link() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        // shortcuts only has an ios deep link + a relative /shortcuts web path
        let url = open_url_for(p, "shortcuts", "ESV");
        assert!(url.starts_with("https://"), "{url}");
        assert!(!url.starts_with("shortcuts://"));
    }

    #[test]
    fn simple_adapter_expands_from_bundled_links() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let url = expand_adapter_url("bible-gateway", p, "ESV").expect("bible-gateway template");
        assert!(url.starts_with("https://www.biblegateway.com/passage/?search="));
        assert!(url.contains("John%203%3A16"));
    }

    #[test]
    fn logos_expands_web_template_locally() {
        let p = Passage::from_refs(&[rom(3), rom(5)]).unwrap();
        let url = open_url_for(p, "logos", "ESV");
        assert!(
            url.starts_with("https://app.logos.com/") || url.starts_with("https://route.bible/"),
            "{url}"
        );
        assert!(!url.contains("ref.ly")); // ios/android deep link
    }

    #[test]
    fn fallback_open_url_points_at_route_bible() {
        let p = Passage::from_refs(&[rom(3), rom(5)]).unwrap();
        assert_eq!(p.fallback_open_url("mailto"), "https://route.bible/rom.5.3-5?app=mailto&mode=web");
    }

    #[test]
    fn all_bundled_adapters_yield_https_on_desktop() {
        let passages = [
            Passage::from_refs(&[john(16)]).unwrap(),
            Passage::from_refs(&[rom(3), rom(5)]).unwrap(),
            Passage::chapter_only(BookId(42), 3),
        ];
        let catalog = Catalog::bundled();
        let mut fallback_only: Vec<&str> = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        for adapter in &catalog.items {
            if adapter.status == "disabled" {
                continue;
            }
            let mut used_fallback_all = true;
            for p in passages {
                let url = open_url_for(p, &adapter.id, "ESV");
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    failures.push(format!("{} → non-http {url}", adapter.id));
                }
                if url.contains("{{") {
                    failures.push(format!("{} → unexpanded {url}", adapter.id));
                }
                if !uses_launcher_fallback(&adapter.id, p, "ESV") {
                    used_fallback_all = false;
                }
            }
            if used_fallback_all {
                fallback_only.push(adapter.id.as_str());
            }
        }
        assert!(failures.is_empty(), "failures:\n{}", failures.join("\n"));
        // mailto has no web platform link — must fall back.
        assert!(
            fallback_only.contains(&"mailto"),
            "expected mailto in fallback-only list, got {fallback_only:?}"
        );
        eprintln!("fallback-only adapters: {fallback_only:?}");
    }
}
