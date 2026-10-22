//! Open a passage in another Bible app or site via [route.bible](https://route.bible) adapters.
//!
//! The adapter list and link templates are **static**: vendored from route.bible's adapters feed
//! into `crates/core/data/` (optionally refreshed at build time with `ROUTE_BIBLE_ADAPTERS=refresh`)
//! and embedded at compile time. Choosing an entry expands a local template for the current
//! verse/range — no runtime network. Desktop only uses links whose `platforms` include `web`
//! (or a desktop OS); mobile-only deep links are never opened. Specialty templates that still
//! cannot expand fall back to a route.bible launcher URL.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::books::{ChapterIdx, VerseRef};
use crate::model::Translation;
use crate::open_in_context;

const API_BASE: &str = "https://route.bible";

/// route.bible catalogType order and human labels (from their launcher UI).
const CATALOG_TYPES: &[(&str, &str)] =
    &[("read", "Read"), ("explore", "Learn"), ("route", "Tools"), ("practice", "Practice")];

/// route.bible catalogSubtype order and labels, keyed by catalogType.
fn catalog_subtypes(catalog_type: &str) -> &'static [(&'static str, &'static str)] {
    match catalog_type {
        "read" => &[("bibles", "Bibles"), ("audio", "Audio")],
        "explore" => &[
            ("ask", "Ask"),
            ("lexicon", "Lexicon"),
            ("commentary", "Commentary"),
            ("guides", "Guides"),
            ("media", "Media"),
        ],
        "route" => &[("note", "Note"), ("share", "Share"), ("translate", "Translate")],
        "practice" => &[("habits", "Habits")],
        _ => &[],
    }
}

/// Links JSON embedded at compile time. `build.rs` stages the vendored snapshot (or a refreshed
/// feed when `ROUTE_BIBLE_ADAPTERS=refresh`) into `OUT_DIR`.
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
    /// `verse_text` is plain scripture from the open translation (see [`passage_plain_text`]).
    pub fn template_vars(self, translation: &str, verse_text: &str) -> BTreeMap<&'static str, String> {
        open_in_context::template_replacements(self, translation, verse_text)
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

impl Adapter {
    /// True when the adapter has a desktop-usable link (web/desktop OS, or unscoped like mailto).
    pub fn visible_on_desktop(&self) -> bool {
        self.links.iter().any(|l| {
            l.usable_on_desktop()
                || l.platforms.is_empty()
                || l.template.starts_with("mailto:")
                || l.template.contains("translate.google.")
                || l.template.contains("translate.kagi.")
        })
    }

    /// Translate destinations (Google / Kagi / catalogSubtype=translate): raw verse text only.
    pub fn is_translate(&self) -> bool {
        self.catalog_subtype.as_deref() == Some("translate")
            || self
                .links
                .iter()
                .any(|l| l.template.contains("translate.google.") || l.template.contains("translate.kagi."))
    }
}

/// Plain verse text for a passage from `t`: verses joined with spaces, no verse numbers.
pub fn passage_plain_text(t: &Translation, passage: Passage) -> String {
    let Some(chapter) = ChapterIdx::new(passage.book, passage.chapter) else {
        return String::new();
    };
    if !passage.has_verse() {
        return t
            .verses_in(chapter)
            .map(|v| v.plain_text())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    (passage.verse_start..=passage.verse_end)
        .filter_map(|n| t.find(VerseRef::new(passage.book, passage.chapter, n)))
        .map(|v| v.plain_text())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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

/// One labelled group in the Open in › flyout (route.bible catalog subtype).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSection<'a> {
    pub label: &'static str,
    pub items: Vec<&'a Adapter>,
}

impl Catalog {
    /// Embedded snapshot — no disk I/O, no network at runtime.
    pub fn bundled() -> Self {
        let file = bundled_links();
        Self { items: file.adapters }
    }

    /// Desktop-visible adapters grouped by route.bible catalog type → subtype order.
    /// Empty groups (after filtering mobile-only) are omitted. Within a section: featured by
    /// rank, then A–Z.
    pub fn menu_sections(&self) -> Vec<MenuSection<'_>> {
        let mut sections = Vec::new();
        for &(ctype, _type_label) in CATALOG_TYPES {
            for &(subtype, label) in catalog_subtypes(ctype) {
                let mut items: Vec<&Adapter> = self
                    .items
                    .iter()
                    .filter(|a| a.status != "disabled")
                    .filter(|a| a.visible_on_desktop())
                    .filter(|a| {
                        a.catalog_type.as_deref() == Some(ctype)
                            && a.catalog_subtype.as_deref() == Some(subtype)
                    })
                    .collect();
                if items.is_empty() {
                    continue;
                }
                items.sort_by(|a, b| match (a.featured_rank, b.featured_rank) {
                    (Some(x), Some(y)) => {
                        x.cmp(&y).then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
                    }
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()),
                });
                sections.push(MenuSection { label, items });
            }
        }
        // Any active desktop adapter missing catalog fields — append under "Other".
        let grouped_ids: std::collections::HashSet<&str> =
            sections.iter().flat_map(|s| s.items.iter().map(|a| a.id.as_str())).collect();
        let mut other: Vec<&Adapter> = self
            .items
            .iter()
            .filter(|a| a.status != "disabled" && a.visible_on_desktop())
            .filter(|a| !grouped_ids.contains(a.id.as_str()))
            .collect();
        if !other.is_empty() {
            other.sort_by_key(|a| a.name.to_ascii_lowercase());
            sections.push(MenuSection { label: "Other", items: other });
        }
        sections
    }

    /// Flat list in section order (for tests / callers that do not draw headers).
    pub fn menu_items(&self) -> Vec<&Adapter> {
        self.menu_sections().into_iter().flat_map(|s| s.items).collect()
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
pub fn expand_adapter_url(
    adapter_id: &str,
    passage: Passage,
    translation: &str,
    verse_text: &str,
) -> Option<String> {
    let catalog = Catalog::bundled();
    let adapter = catalog.items.iter().find(|a| a.id == adapter_id)?;
    let mut vars = passage.template_vars(translation, verse_text);
    // Translate templates still say `{{actionBody}}` in the feed — fill with raw verse text only.
    if adapter.is_translate() {
        vars.insert("actionBody", open_in_context::encoded_raw_text(verse_text));
    }
    let usable: Vec<&LinkTemplate> =
        adapter.links.iter().filter(|l| l.usable_on_desktop() || l.platforms.is_empty()).collect();
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
pub fn uses_launcher_fallback(
    adapter_id: &str,
    passage: Passage,
    translation: &str,
    verse_text: &str,
) -> bool {
    expand_adapter_url(adapter_id, passage, translation, verse_text).is_none()
}

/// Best URL for opening `adapter_id` with `passage` on desktop: local web template expansion,
/// else the route.bible launcher. Never returns a mobile-only deep link.
pub fn open_url_for(passage: Passage, adapter_id: &str, translation: &str, verse_text: &str) -> String {
    expand_adapter_url(adapter_id, passage, translation, verse_text)
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

    const JN316_KJV: &str = "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.";
    const JN316_BSB: &str = "For God so loved the world that He gave His one and only Son, that everyone who believes in Him shall not perish but have eternal life.";

    #[test]
    fn expand_template_fills_known_placeholders() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let vars = p.template_vars("ESV", "");
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
        let vars = p.template_vars("ESV", "");
        assert!(expand_template("https://x.test/{{notARealVar}}/{{passage}}", &vars).is_none());
    }

    #[test]
    fn menu_sections_follow_route_bible_group_order() {
        let c = Catalog::bundled();
        assert!(c.items.len() >= 50, "bundled links should ship the public adapters");
        let sections = c.menu_sections();
        let labels: Vec<&str> = sections.iter().map(|s| s.label).collect();
        // Subtype order within catalogType order; empty groups omitted.
        let expected_prefix = ["Bibles", "Audio", "Ask", "Lexicon"];
        for (i, want) in expected_prefix.iter().enumerate() {
            assert_eq!(labels.get(i), Some(want), "labels={labels:?}");
        }
        assert!(labels.contains(&"Translate"), "{labels:?}");
        assert!(labels.contains(&"Habits"), "{labels:?}");
        // Featured YouVersion stays in Bibles, ranked first within that section.
        let bibles = sections.iter().find(|s| s.label == "Bibles").unwrap();
        assert_eq!(bibles.items[0].id, "bible-app");
        assert_eq!(bibles.items[0].featured_rank, Some(1));
        // No empty sections.
        assert!(sections.iter().all(|s| !s.items.is_empty()));
        // Translate group has google + kagi.
        let tr = sections.iter().find(|s| s.label == "Translate").unwrap();
        let ids: Vec<&str> = tr.items.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"google-translate") && ids.contains(&"kagi-translate"), "{ids:?}");
    }

    #[test]
    fn bible_app_opens_https_youversion_web_on_desktop() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let url = open_url_for(p, "bible-app", "ESV", "");
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
        let url = open_url_for(p, "shortcuts", "ESV", "");
        assert!(url.starts_with("https://"), "{url}");
        assert!(!url.starts_with("shortcuts://"));
    }

    #[test]
    fn simple_adapter_expands_from_bundled_links() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let url = expand_adapter_url("bible-gateway", p, "ESV", "").expect("bible-gateway template");
        assert!(url.starts_with("https://www.biblegateway.com/passage/?search="));
        assert!(url.contains("John%203%3A16"));
    }

    #[test]
    fn logos_expands_web_template_locally() {
        let p = Passage::from_refs(&[rom(3), rom(5)]).unwrap();
        let url = open_url_for(p, "logos", "ESV", "");
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
    fn google_and_kagi_translate_use_raw_verse_text() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        for (id, text, tr) in [
            ("google-translate", JN316_KJV, "KJV"),
            ("kagi-translate", JN316_KJV, "KJV"),
            ("google-translate", JN316_BSB, "BSB"),
            ("kagi-translate", JN316_BSB, "BSB"),
        ] {
            let url = open_url_for(p, id, tr, text);
            let encoded = open_in_context::encoded_raw_text(text);
            assert!(url.contains(&encoded), "{id} {tr} missing verse text: {url}");
            assert!(!url.contains("route.bible"), "{id} must not put route.bible in translate text: {url}");
            // Attribution line must not appear in translate URLs.
            assert!(!url.contains("%E2%80%94"), "{id} has em dash: {url}");
        }
    }

    #[test]
    fn action_body_includes_verse_text_and_attribution() {
        let p = Passage::from_refs(&[john(16)]).unwrap();
        let vars = p.template_vars("KJV", JN316_KJV);
        let body = vars.get("actionBody").unwrap();
        // Encoded "{text}\n— John 3:16 (KJV)" — no route.bible URL.
        assert!(body.starts_with(&open_in_context::encoded_raw_text(JN316_KJV)));
        assert!(body.contains("%E2%80%94"), "expected em dash in actionBody: {body}");
        assert!(body.contains("John%203%3A16"));
        assert!(!body.contains("route.bible"));
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
                let url = open_url_for(p, &adapter.id, "ESV", "sample verse text");
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    failures.push(format!("{} → non-http {url}", adapter.id));
                }
                if url.contains("{{") {
                    failures.push(format!("{} → unexpanded {url}", adapter.id));
                }
                if !uses_launcher_fallback(&adapter.id, p, "ESV", "sample verse text") {
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
