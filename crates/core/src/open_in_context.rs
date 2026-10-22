//! Template-variable context for route.bible adapter link expansion.
//!
//! Ported from `@route-bible/adapters` (`packages/route-bible-adapters/src/context.ts`). The
//! per-book spellings, translation maps and OpenBible chapter lists are generated from route.bible's
//! source into [`crate::open_in_tables`]; `tests/open_in_parity.rs` checks every value against
//! route.bible's own output. Keeps Open in › offline: every `{{placeholder}}` the vendored
//! `adapters.links.json` uses is filled locally.

use std::collections::BTreeMap;

use crate::books::BOOKS;
use crate::open_in::Passage;
use crate::open_in_tables::{self as tables, BOOK_VARIANTS, BookVariants};

const ROUTE_BIBLE_ORIGIN: &str = "https://route.bible";

fn variants(code: &str) -> &'static BookVariants {
    BOOK_VARIANTS.iter().find(|b| b.code == code).unwrap_or(&BOOK_VARIANTS[0])
}

/// Book position in canonical order (0-based), as route.bible's `OSIS_BOOK_ORDER`.
fn book_order(code: &str) -> usize {
    BOOKS.iter().position(|b| b.code == code).unwrap_or(0)
}

/// route.bible's `normalizeTranslationCode`: alphanumerics only, uppercase, `BSB` when empty.
pub fn normalize_translation(input: &str) -> String {
    let normalized: String = input.trim().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if normalized.is_empty() { "BSB".into() } else { normalized.to_ascii_uppercase() }
}

/// JavaScript `encodeURIComponent`.
pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RangeType {
    Chapter,
    Single,
    SameChapter,
}

fn range_type(p: Passage) -> RangeType {
    if !p.has_verse() {
        RangeType::Chapter
    } else if p.verse_start == p.verse_end {
        RangeType::Single
    } else {
        RangeType::SameChapter
    }
}

fn code(p: Passage) -> &'static str {
    p.book.info().code
}

/// `formatPassageForBibleGateway` (same-chapter passages only; bible-rs never builds others).
fn format_bible_gateway(p: Passage) -> String {
    let name = variants(code(p)).name;
    match range_type(p) {
        RangeType::Chapter => format!("{name} {}", p.chapter),
        RangeType::Single => format!("{name} {}:{}", p.chapter, p.verse_start),
        RangeType::SameChapter => format!("{name} {}:{}-{}", p.chapter, p.verse_start, p.verse_end),
    }
}

fn format_literal_word(p: Passage) -> String {
    let name = variants(code(p)).name;
    match range_type(p) {
        RangeType::Chapter => format!("{name} {}", p.chapter),
        RangeType::Single => format!("{name} {} {}", p.chapter, p.verse_start),
        RangeType::SameChapter => format!("{name} {} {}-{}", p.chapter, p.verse_start, p.verse_end),
    }
}

fn format_logos_ref_ly(p: Passage) -> String {
    let c = code(p);
    let book = if c == "PSA" { "Psalm" } else { variants(c).logos_ref_ly };
    let ch = p.chapter;
    match range_type(p) {
        RangeType::Chapter => format!("{book}{ch}"),
        RangeType::Single => format!("{book}{ch}.{}", p.verse_start),
        RangeType::SameChapter => format!("{book}{ch}.{}-{}", p.verse_start, p.verse_end),
    }
}

fn logos_book_number(code: &str) -> usize {
    let n = book_order(code) + 1;
    if n >= 40 { n + 21 } else { n }
}

fn format_logos_data_reference(p: Passage, translation: &str) -> String {
    let t = translation.trim().to_ascii_lowercase();
    let t = if t.is_empty() { "esv".to_string() } else { t };
    let book_number = logos_book_number(code(p));
    let chapter = p.chapter;
    let start = if p.has_verse() { p.verse_start } else { 1 };
    if range_type(p) == RangeType::SameChapter {
        format!("bible+{t}.{book_number}.{chapter}.{start}-{}", p.verse_end)
    } else {
        format!("bible+{t}.{book_number}.{chapter}.{start}")
    }
}

fn verse_range_fragment(p: Passage) -> String {
    match range_type(p) {
        RangeType::Chapter => String::new(),
        RangeType::Single => format!("#{}", p.verse_start),
        RangeType::SameChapter => format!("#{}-{}", p.verse_start, p.verse_end),
    }
}

fn bolls_verse_path(p: Passage) -> String {
    match range_type(p) {
        RangeType::Chapter => "/".into(),
        RangeType::Single => format!("/{}/", p.verse_start),
        RangeType::SameChapter => format!("/{}-{}/", p.verse_start, p.verse_end),
    }
}

fn type_the_word_path(p: Passage) -> String {
    let base = format!("{}_{}", variants(code(p)).type_the_word, p.chapter);
    match range_type(p) {
        RangeType::Chapter => base,
        RangeType::Single => format!("{base}:{}", p.verse_start),
        RangeType::SameChapter => format!("{base}:{}-{}", p.verse_start, p.verse_end),
    }
}

/// `<book><sep><chapter>`, plus `<sep><verse>` for a single verse (ranges open the chapter).
fn book_chapter_single_verse(book: &str, p: Passage, sep: &str) -> String {
    if range_type(p) == RangeType::Single {
        format!("{book}{sep}{}{sep}{}", p.chapter, p.verse_start)
    } else {
        format!("{book}{sep}{}", p.chapter)
    }
}

fn bible_hub_path(p: Passage) -> String {
    let book = variants(code(p)).bible_hub;
    if range_type(p) == RangeType::Single {
        format!("{book}/{}-{}.htm", p.chapter, p.verse_start)
    } else {
        format!("{book}/{}.htm", p.chapter)
    }
}

fn bible_study_tools_path(p: Passage) -> String {
    let book = variants(code(p)).slug;
    if range_type(p) == RangeType::Single {
        format!("{book}/{}-{}.html", p.chapter, p.verse_start)
    } else {
        format!("{book}/{}.html", p.chapter)
    }
}

fn literal_word_path(p: Passage) -> String {
    book_chapter_single_verse(&(book_order(code(p)) + 1).to_string(), p, "/")
}

fn simply_my_bible_path(p: Passage, translation: &str) -> String {
    let t = tables::simply_my_bible(translation).unwrap_or("bsb");
    let anchor = if p.has_verse() { format!("#v{}", p.verse_start) } else { String::new() };
    format!("{t}/{}/{}{anchor}", variants(code(p)).slug, p.chapter)
}

fn bible_ref_path(p: Passage) -> String {
    let name = variants(code(p)).name;
    let stripped: String =
        name.chars().filter(|c| !matches!(c, '\'' | '"' | '.' | ',' | '(' | ')')).collect();
    let book_segment = stripped.split_whitespace().collect::<Vec<_>>().join("-");
    let verse_book = if code(p) == "PSA" { "Psalm".to_string() } else { book_segment.clone() };
    let chapter = p.chapter;
    if range_type(p) == RangeType::Single {
        format!("{book_segment}/{chapter}/{verse_book}-{chapter}-{}.html", p.verse_start)
    } else {
        format!("{book_segment}/{chapter}/{book_segment}-chapter-{chapter}.html")
    }
}

fn step_bible_query(p: Passage, translation: &str) -> String {
    let t = tables::step_bible(translation).unwrap_or("ESV");
    let book = variants(code(p)).step_bible;
    match range_type(p) {
        RangeType::Chapter => format!("version={t}@reference={book}.{}", p.chapter),
        RangeType::Single => format!("version={t}@reference={book}.{}.{}", p.chapter, p.verse_start),
        RangeType::SameChapter => {
            format!("version={t}@reference={book}.{}.{}-{}", p.chapter, p.verse_start, p.verse_end)
        }
    }
}

fn bible_project_path(p: Passage) -> String {
    let guide = variants(code(p)).bible_project_guide;
    if guide.is_empty() {
        format!("search/?q={}", encode_uri_component(&format_bible_gateway(p)))
    } else {
        format!("guides/{guide}/")
    }
}

fn open_bible_geo_path(p: Passage) -> String {
    let slug = variants(code(p)).open_bible_geo;
    if slug.is_empty() {
        return String::new();
    }
    if tables::open_bible_geo_chapters(slug).contains(&p.chapter) {
        format!("{slug}.{}", p.chapter)
    } else {
        slug.to_string()
    }
}

/// route.bible's per-adapter UTM tags (`ADAPTER_UTM_MAP`).
fn adapter_utm(adapter_id: Option<&str>) -> Option<(&'static str, &'static str)> {
    Some(match adapter_id? {
        "obsidian" => ("obsidian", "note"),
        "bear" => ("bear", "note"),
        "day-one" => ("day-one", "note"),
        "drafts" => ("drafts", "note"),
        "mailto" => ("email", "share"),
        "telegram" => ("telegram", "share"),
        "whatsapp" => ("whatsapp", "share"),
        "x" => ("x", "share"),
        "chatgpt" => ("chatgpt", "prompt"),
        "claude" => ("claude", "prompt"),
        "grok" => ("grok", "prompt"),
        "t3-chat" => ("t3-chat", "prompt"),
        "perplexity" => ("perplexity", "prompt"),
        "kagi-translate" => ("kagi-translate", "translate"),
        "google-translate" => ("google-translate", "translate"),
        "todoist" => ("todoist", "task"),
        "things-3" => ("things-3", "task"),
        "omnifocus" => ("omnifocus", "task"),
        "anki" => ("anki", "flashcard"),
        "shortcuts" => ("shortcuts", "automation"),
        "calendar" => ("calendar", "subscription"),
        _ => return None,
    })
}

/// URLSearchParams-style value encoding (spaces as `+`).
fn form_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn route_bible_canonical_url(p: Passage, utm: Option<(&str, &str)>) -> String {
    let path = p.route_path();
    match utm {
        Some((source, medium)) => format!(
            "{ROUTE_BIBLE_ORIGIN}{path}?utm_source={}&utm_medium={}",
            form_encode(source),
            form_encode(medium)
        ),
        None => format!("{ROUTE_BIBLE_ORIGIN}{path}"),
    }
}

fn route_bible_source_url(p: Passage, translation: &str, (source, medium): (&str, &str)) -> String {
    format!(
        "{ROUTE_BIBLE_ORIGIN}{}?v={}&utm_source={}&utm_medium={}",
        p.route_path(),
        form_encode(translation),
        form_encode(source),
        form_encode(medium)
    )
}

fn route_bible_prompt_url(display: &str, utm_source: &str) -> String {
    format!(
        "{ROUTE_BIBLE_ORIGIN}/?q={}&utm_source={}&utm_medium=prompt",
        encode_uri_component(display),
        encode_uri_component(utm_source)
    )
}

fn scripture_markdown_link(display: &str, utm_source: &str) -> String {
    format!("[{display}]({})", route_bible_prompt_url(display, utm_source))
}

fn build_ai_prompt_body(display: &str, translation: &str, utm_source: &str) -> String {
    let base = format!("{ROUTE_BIBLE_ORIGIN}/?q=");
    let suffix = format!("&utm_source={}&utm_medium=prompt", encode_uri_component(utm_source));
    [
        "I have a question about a Bible passage. Use your knowledge of the text to answer.".to_string(),
        String::new(),
        "RULE: Every Scripture reference you write must be a Markdown link to route.bible using OSIS notation in the URL. Never omit the book — even when context implies it. Never write a bare \"verse 4\" or \"chapter 2\"; always include the full OSIS reference.".to_string(),
        "Examples:".to_string(),
        format!("[John 3:16]({base}JHN.3.16{suffix})"),
        format!("[Revelation 2:1-7]({base}REV.2.1-REV.2.7{suffix})"),
        format!("[Genesis 1]({base}GEN.1{suffix})"),
        format!("[Psalm 23:1-6]({base}PSA.23.1-PSA.23.6{suffix})"),
        String::new(),
        "Passage in question:".to_string(),
        scripture_markdown_link(display, utm_source),
        String::new(),
        format!("Translation: {translation}"),
    ]
    .join("\n")
}

/// Soft cap for GET query values (translate / capture URLs). Longer ranges truncate with an ellipsis.
pub const MAX_URL_TEXT_CHARS: usize = 1200;

/// Truncate `s` for embedding in a URL query, appending `…` when clipped.
pub fn truncate_for_url(s: &str) -> String {
    let n = s.chars().count();
    if n <= MAX_URL_TEXT_CHARS {
        return s.to_string();
    }
    let mut out: String = s.chars().take(MAX_URL_TEXT_CHARS.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// `actionBody` for notes / mail / capture: verse text, then an attribution line.
/// route.bible's feed templates still use `{{actionBody}}` for translate URLs too — callers that
/// expand a translate adapter should override with [`encoded_raw_text`].
pub fn action_body_with_text(verse_text: &str, display: &str, translation: &str) -> String {
    let text = truncate_for_url(verse_text.trim());
    if text.is_empty() {
        format!("{display} ({translation})")
    } else {
        format!("{text}\n— {display} ({translation})")
    }
}

/// Max characters of passage text placed in `{{verseText}}` (route.bible `VERSE_TEXT_MAX_LENGTH`).
pub const VERSE_TEXT_MAX_LENGTH: usize = 500;

/// Strips a leading verse number from one line: `16 `, `16. `, `16) `, `3:16 `, `v16 `, `v. 16 `
/// (only when text follows), as route.bible's `buildVerseQueryText` does.
fn strip_leading_verse_number(line: &str) -> &str {
    let t = line.trim_start();
    let mut rest = t;
    if let Some(r) = rest.strip_prefix(['v', 'V']) {
        let r = r.strip_prefix('.').unwrap_or(r);
        rest = r.trim_start();
    }
    let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
    let n = digits(rest);
    if !(1..=3).contains(&n) {
        return line;
    }
    rest = &rest[n..];
    if let Some(r) = rest.strip_prefix(':') {
        let m = digits(r);
        if (1..=3).contains(&m) {
            rest = &r[m..];
        }
    }
    rest = rest.strip_prefix(['.', ')']).unwrap_or(rest);
    let after = rest.trim_start();
    if after.len() == rest.len() || after.is_empty() || after.starts_with(|c: char| c.is_ascii_digit()) {
        return line;
    }
    after
}

/// Replaces bracketed verse numbers (`[16]`, `[ 16 ]`) with a space.
fn drop_bracketed_numbers(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(i) = rest.find('[') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        let inner = tail.trim_start();
        let n = inner.bytes().take_while(u8::is_ascii_digit).count();
        let after = inner[n..].trim_start();
        if (1..=3).contains(&n) && after.starts_with(']') {
            out.push(' ');
            rest = &after[1..];
        } else {
            out.push('[');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// Plain passage text for search adapters (`{{verseText}}`), matching route.bible's
/// `buildVerseQueryText`: verse numbers and footnote marks stripped, whitespace collapsed, capped
/// at [`VERSE_TEXT_MAX_LENGTH`] on a word boundary. Not encoded.
pub fn verse_query_text(text: &str) -> String {
    let cleaned = text
        .lines()
        .map(|line| {
            let line = drop_bracketed_numbers(line);
            strip_leading_verse_number(&line).replace(['¶', '*', '†', '‡'], " ")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let max = VERSE_TEXT_MAX_LENGTH;
    if cleaned.chars().count() <= max {
        return cleaned;
    }
    let slice: String = cleaned.chars().take(max).collect();
    match slice.rfind(' ') {
        Some(i) if slice[..i].chars().count() as f32 > max as f32 * 0.6 => slice[..i].trim().to_string(),
        _ => slice.trim().to_string(),
    }
}

/// Percent-encoded raw verse text for translate URL `text=` params.
pub fn encoded_raw_text(verse_text: &str) -> String {
    encode_uri_component(&truncate_for_url(verse_text.trim()))
}

fn obsidian_note_name(display: &str) -> String {
    let sanitized = display.replace(':', ".").replace(['<', '>', '"', '/', '\\', '|', '?', '*'], " ");
    sanitized.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `JSON.stringify` for the YAML scalars in note front matter.
fn yaml_scalar(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Today's UTC date, `YYYY-MM-DD` (route.bible stamps notes with the capture date).
fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Howard Hinnant's civil_from_days.
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn obsidian_note_content(p: Passage, display: &str, source_url: &str) -> String {
    let v = variants(code(p));
    let lines = [
        "---".to_string(),
        format!("source: {}", yaml_scalar(&format!("[route.bible]({source_url})"))),
        format!("date: {}", yaml_scalar(&today_utc())),
        format!("reference: {}", yaml_scalar(display)),
        format!("osis: {}", yaml_scalar(&p.canonical())),
        format!("book: {}", yaml_scalar(v.name)),
        "bridges:".to_string(),
        format!("  - {}", yaml_scalar(&format!("[[{} {}]]", v.name, p.chapter))),
        format!("  - {}", yaml_scalar(&format!("[[{}]]", v.name))),
        "tags:".to_string(),
        format!("  - {}", yaml_scalar("route-bible")),
        format!("  - {}", yaml_scalar("scripture")),
        format!("  - {}", yaml_scalar(&format!("book/{}", v.slug))),
        "---".to_string(),
        String::new(),
        format!("# {display}"),
        String::new(),
        format!("Source: [route.bible]({source_url})"),
        String::new(),
        "## Notes".to_string(),
        String::new(),
    ];
    lines.join("\n")
}

fn obsidian_reading_log_base() -> &'static str {
    "filters:\n  and:\n    - file.hasTag(\"route-bible\")\nviews:\n  - type: table\n    name: Log\n    order:\n      - file.name\n      - date\n      - source\n      - file.mtime\n      - book\n      - reference\n      - tags"
}

/// Build substitution values for every `{{placeholder}}` used by the vendored links snapshot,
/// matching route.bible's `buildAdapterTemplateReplacements(buildAdapterTemplateContext(…))`.
/// Values that belong in query strings are already `encodeURIComponent`-encoded.
///
/// `verse_text` is the open translation's plain verse text (spaces between verses, no numbers);
/// `actionBody` carries it (route.bible, without the text, sends a link instead).
pub fn template_replacements(
    passage: Passage,
    translation: &str,
    verse_text: &str,
) -> BTreeMap<&'static str, String> {
    template_replacements_for(passage, translation, verse_text, None)
}

/// [`template_replacements`] for one adapter, whose id picks route.bible's UTM tags.
pub fn template_replacements_for(
    passage: Passage,
    translation: &str,
    verse_text: &str,
    adapter_id: Option<&str>,
) -> BTreeMap<&'static str, String> {
    let translation = normalize_translation(translation);
    let t = translation.as_str();
    let c = code(passage);
    let v = variants(c);
    let (yv_id, yv_code) = tables::youversion(t).unwrap_or(("3034", "BSB"));
    let display = format_bible_gateway(passage);
    let utm = adapter_utm(adapter_id);
    let route_url = route_bible_canonical_url(passage, utm);
    let source_url = route_bible_source_url(passage, t, utm.unwrap_or(("obsidian", "note")));
    let ai_utm_source = utm.map(|u| u.0).unwrap_or("chatgpt");
    let action_title = format!("{display} ({t})");
    let text_for_url = truncate_for_url(verse_text.trim());
    let action_body = action_body_with_text(&text_for_url, &display, t);
    let chapter = passage.chapter.to_string();
    let verse = if passage.has_verse() { passage.verse_start.to_string() } else { "1".into() };
    let todoist = format!("{} ({t})", display.replace(':', "."));
    let order = book_order(c);

    let mut m = BTreeMap::new();
    // Encoded (query-safe) values — matching route.bible's buildAdapterTemplateReplacements.
    m.insert("passage", encode_uri_component(&passage.canonical()));
    m.insert("passageLower", encode_uri_component(&passage.canonical().to_ascii_lowercase()));
    m.insert("displayPassage", encode_uri_component(&display));
    m.insert("translation", encode_uri_component(t));
    m.insert("translationLower", encode_uri_component(&t.to_ascii_lowercase()));
    m.insert("routeUrl", encode_uri_component(&route_url));
    m.insert(
        "scriptureMarkdownLink",
        encode_uri_component(&scripture_markdown_link(&display, ai_utm_source)),
    );
    m.insert("shareText", encode_uri_component(&format!("{display} {route_url}")));
    m.insert("actionTitle", encode_uri_component(&action_title));
    m.insert("actionBody", encode_uri_component(&action_body));
    // Search adapters (Hidden Arrow): the local text, else the reference.
    let query = verse_query_text(verse_text);
    m.insert("verseText", encode_uri_component(if query.is_empty() { &display } else { &query }));
    // Raw verse text only (for callers); translate expansion overrides actionBody with it.
    m.insert("passageText", encode_uri_component(&text_for_url));
    m.insert("aiPromptBody", encode_uri_component(&build_ai_prompt_body(&display, t, ai_utm_source)));
    m.insert("todoistTaskTitle", encode_uri_component(&todoist));
    m.insert("youversionVersionId", encode_uri_component(yv_id));
    m.insert("youversionTranslation", encode_uri_component(yv_code));
    m.insert("lang", "en".into());
    m.insert("bollsTranslation", encode_uri_component(tables::bolls(t).unwrap_or("ESV")));
    m.insert("typeTheWordTranslation", encode_uri_component(tables::type_the_word(t).unwrap_or("esv")));
    m.insert("studyLightTranslation", encode_uri_component(tables::study_light(t).unwrap_or("kjv")));
    m.insert("studyBibleTranslation", encode_uri_component(tables::study_bible(t).unwrap_or("YLT")));
    m.insert("studyLightOriginalLanguage", (if order >= 39 { "greek" } else { "hebrew" }).into());
    m.insert("bibleIsTranslation", encode_uri_component(tables::bible_is(t).unwrap_or("engesv")));
    m.insert("bibliaTranslation", encode_uri_component(tables::biblia(t).unwrap_or("esv")));
    m.insert("literalWordTranslation", encode_uri_component(tables::literal_word(t).unwrap_or("esv")));
    m.insert("bookSlug", encode_uri_component(v.slug));
    m.insert("bookCode", encode_uri_component(v.blue_letter_bible));
    m.insert("bookNumber", (order + 1).to_string());
    m.insert("chapter", chapter.clone());
    m.insert("verse", verse);
    m.insert("chapterPath", encode_uri_component(&format!("{}-{chapter}", v.enduring_word)));
    m.insert("logosDataReference", encode_uri_component(&format_logos_data_reference(passage, "esv")));
    m.insert("logosRefLyReference", encode_uri_component(&format_logos_ref_ly(passage)));
    m.insert("literalWordQuery", encode_uri_component(&format_literal_word(passage)));
    m.insert("stepBibleQuery", encode_uri_component(&step_bible_query(passage, t)));
    m.insert("obsidianNoteName", encode_uri_component(&obsidian_note_name(&display)));
    m.insert(
        "obsidianNoteContent",
        encode_uri_component(&obsidian_note_content(passage, &display, &source_url)),
    );
    m.insert("obsidianReadingLogBaseContent", encode_uri_component(obsidian_reading_log_base()));
    // Path segments — left unencoded (already URL-safe).
    m.insert("typeTheWordPath", type_the_word_path(passage));
    m.insert("sermonAudioPath", book_chapter_single_verse(v.sermon_audio, passage, "/"));
    m.insert("bibleIsPath", book_chapter_single_verse(&v.bible_is.to_ascii_lowercase(), passage, "/"));
    m.insert("biblicaPath", book_chapter_single_verse(v.biblica, passage, "/"));
    m.insert("literalWordPath", literal_word_path(passage));
    m.insert("verseRange", verse_range_fragment(passage));
    m.insert("versePath", bolls_verse_path(passage));
    m.insert("bibleRefPath", bible_ref_path(passage));
    m.insert("bibleHubPath", bible_hub_path(passage));
    m.insert("bibleStudyToolsPath", bible_study_tools_path(passage));
    m.insert("catenaPath", book_chapter_single_verse(v.catena, passage, "/"));
    m.insert("simplyMyBiblePath", simply_my_bible_path(passage, t));
    m.insert("bibleProjectPath", bible_project_path(passage));
    m.insert("openBibleGeoPath", open_bible_geo_path(passage));
    m
}
