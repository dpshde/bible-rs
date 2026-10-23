//! "Open in ›": links that open a passage in another Bible app or site.
//!
//! A short, hand-picked list. Each link is built here from the passage, the way
//! [route.bible](https://route.bible)'s adapters build theirs (same book spellings, range forms
//! and translation codes), so nothing is fetched at runtime.

use crate::books::{BookId, FIRST_NT_BOOK, VerseRef};

/// One destination: its id (route.bible's adapter id) and the label shown in the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct App {
    pub id: &'static str,
    pub name: &'static str,
}

/// The destinations, in menu order.
pub const APPS: [App; 10] = [
    App { id: "bible-app", name: "Bible App" },
    App { id: "literal-word", name: "Literal Word" },
    App { id: "verse-notes", name: "Verse Notes" },
    App { id: "blue-letter-bible", name: "Blue Letter Bible" },
    App { id: "step-bible", name: "STEP Bible" },
    App { id: "logos", name: "Logos" },
    App { id: "desiring-god", name: "Desiring God" },
    App { id: "enduring-word", name: "Enduring Word" },
    App { id: "tgc-commentary", name: "The Gospel Coalition" },
    App { id: "chatgpt", name: "ChatGPT" },
];

/// The starred apps, in the order they were starred. Ids not in [`APPS`] are left out.
pub fn starred(ids: &[String]) -> Vec<App> {
    let mut out: Vec<App> = Vec::new();
    for id in ids {
        if let Some(app) = APPS.iter().find(|a| a.id == id)
            && !out.contains(app)
        {
            out.push(*app);
        }
    }
    out
}

/// The submenu's order: the starred apps first, then the rest in [`APPS`] order.
pub fn menu_order(ids: &[String]) -> Vec<App> {
    let mut out = starred(ids);
    out.extend(APPS.iter().filter(|a| !ids.iter().any(|id| id == a.id)));
    out
}

/// Stars `id`, or unstars it when it already is. New stars go last.
pub fn toggle_star(ids: &mut Vec<String>, id: &str) {
    match ids.iter().position(|s| s == id) {
        Some(i) => {
            ids.remove(i);
        }
        None => ids.push(id.to_string()),
    }
}

/// A same-chapter passage: one verse or an inclusive range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Passage {
    pub book: BookId,
    pub chapter: u8,
    pub start: u8,
    pub end: u8,
}

impl Passage {
    /// From a non-empty selection of verses in one chapter (order does not matter). A selection
    /// across chapters has no passage.
    pub fn from_refs(refs: &[VerseRef]) -> Option<Self> {
        let first = *refs.first()?;
        if refs.iter().any(|r| r.book != first.book || r.chapter != first.chapter) {
            return None;
        }
        let start = refs.iter().map(|r| r.verse).min()?;
        let end = refs.iter().map(|r| r.verse).max()?;
        Some(Self { book: first.book, chapter: first.chapter, start, end })
    }

    fn is_range(self) -> bool {
        self.end > self.start
    }

    /// `JHN.3.16` or `JHN.3.16-18`.
    fn osis(self) -> String {
        let code = self.book.info().code;
        if self.is_range() {
            format!("{code}.{}.{}-{}", self.chapter, self.start, self.end)
        } else {
            format!("{code}.{}.{}", self.chapter, self.start)
        }
    }

    /// `John 3:16` or `John 3:16-18`.
    fn display(self) -> String {
        let name = self.book.name();
        if self.is_range() {
            format!("{name} {}:{}-{}", self.chapter, self.start, self.end)
        } else {
            format!("{name} {}:{}", self.chapter, self.start)
        }
    }

    /// `1 Samuel` → `1-samuel`.
    fn slug(self) -> String {
        self.book.name().to_ascii_lowercase().replace(' ', "-")
    }
}

/// The link that opens `passage` in the app with `id`, in `translation` where the app has it
/// (otherwise its own default). `None` for an unknown id.
pub fn url(id: &str, passage: Passage, translation: &str) -> Option<String> {
    let t = normalize(translation);
    let p = passage;
    let info = p.book.info();
    let (ch, v) = (p.chapter, p.start);
    Some(match id {
        "bible-app" => {
            let (version, code) = youversion(&t);
            format!("https://www.bible.com/bible/{version}/{}.{code}", p.osis())
        }
        // Literal Word opens a verse, or the chapter for a range.
        "literal-word" => {
            let t = literal_word(&t);
            let book = p.book.index() + 1;
            let verse = if p.is_range() { String::new() } else { format!("/{v}") };
            format!("https://app.literalword.com/{t}/{book}/{ch}{verse}")
        }
        "verse-notes" => format!("https://versenotes.org/chapter-by-chapter/{}/{ch}/", p.slug()),
        // Blue Letter Bible opens the first verse; a translation it lacks 404s, so it's left out.
        "blue-letter-bible" => {
            let book = match info.code {
                "RUT" => "rth",
                "EZK" => "eze",
                "JOL" => "joe",
                "NAM" => "nah",
                "MRK" => "mar",
                "PHP" => "phl",
                "1JN" => "1jo",
                "2JN" => "2jo",
                "3JN" => "3jo",
                "JUD" => "jde",
                c => &c.to_ascii_lowercase(),
            };
            let version = if BLB_TRANSLATIONS.contains(&t.as_str()) {
                format!("{}/", t.to_ascii_lowercase())
            } else {
                String::new()
            };
            format!("https://www.blueletterbible.org/{version}{book}/{ch}/{v}/")
        }
        "step-bible" => {
            let book = match info.code {
                "1CH" => "1Chron",
                "2CH" => "2Chron",
                "PSA" => "Psalm",
                "JON" => "Jon",
                "TIT" => "Tit",
                _ => info.osis,
            };
            let verses = if p.is_range() { format!("{v}-{}", p.end) } else { v.to_string() };
            let q = format!("version={}@reference={book}.{ch}.{verses}", step_bible(&t));
            format!("https://www.stepbible.org/?q={}&options=HNVUG", encode(&q))
        }
        // Logos's web reader opens its ESV; its book numbers skip 40–60 for the deuterocanon.
        "logos" => {
            let n = p.book.index() + 1 + if p.book >= FIRST_NT_BOOK { 21 } else { 0 };
            let verses = if p.is_range() { format!("{v}-{}", p.end) } else { v.to_string() };
            let r = format!("bible+esv.{n}.{ch}.{verses}");
            format!("https://app.logos.com/books/LLS%3A1.0.710/references/{}", encode(&r))
        }
        "desiring-god" => format!("https://www.desiringgod.org/search/results?q={}", encode(&p.display())),
        "enduring-word" => {
            let book = if info.code == "PSA" { "psalm".to_string() } else { p.slug() };
            format!("https://enduringword.com/bible-commentary/{book}-{ch}/")
        }
        // TGC splits Psalms into its five books (`/commentary/psalms/` is a 404).
        "tgc-commentary" => {
            let book = if info.code == "PSA" {
                let (first, last) = match ch {
                    ..=41 => (1, 41),
                    42..=72 => (42, 72),
                    73..=89 => (73, 89),
                    90..=106 => (90, 106),
                    _ => (107, 150),
                };
                format!("psalm-{first}-psalm-{last}")
            } else {
                p.slug()
            };
            format!("https://www.thegospelcoalition.org/commentary/{book}/")
        }
        "chatgpt" => format!(
            "https://chatgpt.com/g/g-DcL3nGFWk-bible-bot-christian-companion?prompt={}",
            encode(&chatgpt_prompt(&p.display(), &t))
        ),
        _ => return None,
    })
}

/// Translation codes as route.bible writes them: letters and digits, upper case.
fn normalize(translation: &str) -> String {
    let t: String = translation.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if t.is_empty() { "BSB".into() } else { t.to_ascii_uppercase() }
}

/// YouVersion's version id and code; BSB when it lacks the translation.
fn youversion(t: &str) -> (&'static str, &'static str) {
    match t {
        "KJV" => ("1", "KJV"),
        "NKJV" => ("114", "NKJV"),
        "ESV" => ("59", "ESV"),
        "NIV" => ("111", "NIV"),
        "NLT" => ("116", "NLT"),
        "CSB" => ("1713", "CSB"),
        "NASB95" | "NASB1995" => ("100", "NASB1995"),
        "NRSV" => ("2015", "NRSV-CI"),
        "NRSVUE" => ("3523", "NRSVUE"),
        "NET" => ("107", "NET"),
        "WEB" | "WEBUS" => ("206", "WEBUS"),
        _ => ("3034", "BSB"),
    }
}

/// Literal Word's translation; ESV when it lacks the translation.
fn literal_word(t: &str) -> &'static str {
    match t {
        "KJV" => "kjv",
        "LSB" => "lsb",
        "NASB" | "NASB95" | "NASB1995" => "nasb",
        "NKJV" => "nkjv",
        _ => "esv",
    }
}

/// STEP Bible's translation; ESV when it lacks the translation.
fn step_bible(t: &str) -> &'static str {
    match t {
        "KJV" | "NKJV" => "KJV",
        "NIV" => "NIV",
        "CSB" | "HCSB" => "HCSB",
        "NASB95" | "NASB1995" => "NASB",
        "NRSV" | "NRSVUE" => "NRSV",
        _ => "ESV",
    }
}

/// Translations Blue Letter Bible serves (checked 2026-10-22).
const BLB_TRANSLATIONS: &[&str] = &[
    "AMP", "ASV", "CSB", "DBY", "ESV", "KJV", "LSB", "NASB", "NASB95", "NET", "NIV", "NKJV", "NLT", "RSV",
    "WEB", "YLT",
];

/// route.bible's ChatGPT prompt: the passage, as a link, with the rule that every reference in the
/// answer links back the same way.
fn chatgpt_prompt(display: &str, t: &str) -> String {
    let link = |label: &str, q: &str| {
        format!("[{label}](https://route.bible/?q={q}&utm_source=chatgpt&utm_medium=prompt)")
    };
    [
        "I have a question about a Bible passage. Use your knowledge of the text to answer.".to_string(),
        String::new(),
        "RULE: Every Scripture reference you write must be a Markdown link to route.bible using OSIS notation in the URL. Never omit the book — even when context implies it. Never write a bare \"verse 4\" or \"chapter 2\"; always include the full OSIS reference.".to_string(),
        "Examples:".to_string(),
        link("John 3:16", "JHN.3.16"),
        link("Revelation 2:1-7", "REV.2.1-REV.2.7"),
        link("Genesis 1", "GEN.1"),
        link("Psalm 23:1-6", "PSA.23.1-PSA.23.6"),
        String::new(),
        "Passage in question:".to_string(),
        link(display, &encode(display)),
        String::new(),
        format!("Translation: {t}"),
    ]
    .join("\n")
}

/// JavaScript's `encodeURIComponent`.
fn encode(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn passage(code: &str, chapter: u8, start: u8, end: u8) -> Passage {
        Passage { book: BookId::from_code(code).unwrap(), chapter, start, end }
    }

    /// Each app's link for John 3:16, Romans 5:3–5 and Psalm 23:1, in KJV.
    #[rustfmt::skip]
    const EXPECTED: [(&str, [&str; 3]); 10] = [
        ("bible-app", [
            "https://www.bible.com/bible/1/JHN.3.16.KJV",
            "https://www.bible.com/bible/1/ROM.5.3-5.KJV",
            "https://www.bible.com/bible/1/PSA.23.1.KJV",
        ]),
        ("literal-word", [
            "https://app.literalword.com/kjv/43/3/16",
            "https://app.literalword.com/kjv/45/5",
            "https://app.literalword.com/kjv/19/23/1",
        ]),
        ("verse-notes", [
            "https://versenotes.org/chapter-by-chapter/john/3/",
            "https://versenotes.org/chapter-by-chapter/romans/5/",
            "https://versenotes.org/chapter-by-chapter/psalms/23/",
        ]),
        ("blue-letter-bible", [
            "https://www.blueletterbible.org/kjv/jhn/3/16/",
            "https://www.blueletterbible.org/kjv/rom/5/3/",
            "https://www.blueletterbible.org/kjv/psa/23/1/",
        ]),
        ("step-bible", [
            "https://www.stepbible.org/?q=version%3DKJV%40reference%3DJohn.3.16&options=HNVUG",
            "https://www.stepbible.org/?q=version%3DKJV%40reference%3DRom.5.3-5&options=HNVUG",
            "https://www.stepbible.org/?q=version%3DKJV%40reference%3DPsalm.23.1&options=HNVUG",
        ]),
        ("logos", [
            "https://app.logos.com/books/LLS%3A1.0.710/references/bible%2Besv.64.3.16",
            "https://app.logos.com/books/LLS%3A1.0.710/references/bible%2Besv.66.5.3-5",
            "https://app.logos.com/books/LLS%3A1.0.710/references/bible%2Besv.19.23.1",
        ]),
        ("desiring-god", [
            "https://www.desiringgod.org/search/results?q=John%203%3A16",
            "https://www.desiringgod.org/search/results?q=Romans%205%3A3-5",
            "https://www.desiringgod.org/search/results?q=Psalms%2023%3A1",
        ]),
        ("enduring-word", [
            "https://enduringword.com/bible-commentary/john-3/",
            "https://enduringword.com/bible-commentary/romans-5/",
            "https://enduringword.com/bible-commentary/psalm-23/",
        ]),
        ("tgc-commentary", [
            "https://www.thegospelcoalition.org/commentary/john/",
            "https://www.thegospelcoalition.org/commentary/romans/",
            "https://www.thegospelcoalition.org/commentary/psalm-1-psalm-41/",
        ]),
        ("chatgpt", ["", "", ""]),
    ];

    #[test]
    fn every_app_links_a_verse_and_a_range() {
        let passages = [passage("JHN", 3, 16, 16), passage("ROM", 5, 3, 5), passage("PSA", 23, 1, 1)];
        assert_eq!(APPS.map(|a| a.id), EXPECTED.map(|(id, _)| id), "menu order");
        for (id, want) in EXPECTED.iter().filter(|(id, _)| *id != "chatgpt") {
            for (p, want) in passages.iter().zip(want) {
                assert_eq!(url(id, *p, "kjv").as_deref(), Some(*want), "{id}");
            }
        }
        assert_eq!(url("bible-gateway", passages[0], "kjv"), None);
    }

    #[test]
    fn chatgpt_asks_about_the_passage() {
        let link = url("chatgpt", passage("ROM", 5, 3, 5), "kjv").unwrap();
        let prompt = link
            .strip_prefix("https://chatgpt.com/g/g-DcL3nGFWk-bible-bot-christian-companion?prompt=")
            .unwrap();
        assert!(prompt.starts_with("I%20have%20a%20question"), "{prompt}");
        assert!(
            prompt.contains(
                "%5BRomans%205%3A3-5%5D(https%3A%2F%2Froute.bible%2F%3Fq%3DRomans%25205%253A3-5%26"
            ),
            "{prompt}"
        );
        assert!(prompt.ends_with("Translation%3A%20KJV"), "{prompt}");
    }

    #[test]
    fn book_spellings_follow_each_site() {
        let p = |code| passage(code, 1, 2, 2);
        assert_eq!(
            url("blue-letter-bible", p("EZK"), "esv").unwrap(),
            "https://www.blueletterbible.org/esv/eze/1/2/"
        );
        assert_eq!(
            url("blue-letter-bible", p("JOL"), "esv").unwrap(),
            "https://www.blueletterbible.org/esv/joe/1/2/"
        );
        assert!(url("step-bible", p("1CH"), "esv").unwrap().contains("1Chron.1.2"));
        assert!(url("step-bible", p("TIT"), "esv").unwrap().contains("Tit.1.2"));
        assert!(url("step-bible", p("1CO"), "esv").unwrap().contains("1Cor.1.2"));
        assert_eq!(
            url("verse-notes", p("1SA"), "esv").unwrap(),
            "https://versenotes.org/chapter-by-chapter/1-samuel/1/"
        );
        assert_eq!(
            url("tgc-commentary", p("SNG"), "esv").unwrap(),
            "https://www.thegospelcoalition.org/commentary/song-of-solomon/"
        );
        assert_eq!(
            url("tgc-commentary", passage("PSA", 119, 105, 105), "esv").unwrap(),
            "https://www.thegospelcoalition.org/commentary/psalm-107-psalm-150/"
        );
        // Logos numbers Genesis 1, Malachi 39, Matthew 61, Revelation 87.
        assert!(url("logos", p("MAL"), "esv").unwrap().ends_with("bible%2Besv.39.1.2"));
        assert!(url("logos", p("MAT"), "esv").unwrap().ends_with("bible%2Besv.61.1.2"));
        assert!(url("logos", p("REV"), "esv").unwrap().ends_with("bible%2Besv.87.1.2"));
    }

    #[test]
    fn translations_the_site_lacks_fall_back() {
        let p = passage("JHN", 3, 16, 16);
        assert_eq!(url("bible-app", p, "bsb").unwrap(), "https://www.bible.com/bible/3034/JHN.3.16.BSB");
        assert_eq!(url("bible-app", p, "web").unwrap(), "https://www.bible.com/bible/206/JHN.3.16.WEBUS");
        assert_eq!(url("blue-letter-bible", p, "bsb").unwrap(), "https://www.blueletterbible.org/jhn/3/16/");
        assert_eq!(url("literal-word", p, "bsb").unwrap(), "https://app.literalword.com/esv/43/3/16");
        assert!(url("step-bible", p, "bsb").unwrap().contains("version%3DESV"));
    }

    #[test]
    fn starred_apps_come_first_in_star_order() {
        let mut ids = Vec::new();
        toggle_star(&mut ids, "logos");
        toggle_star(&mut ids, "bible-app");
        toggle_star(&mut ids, "enduring-word");
        toggle_star(&mut ids, "bible-app");
        assert_eq!(ids, ["logos", "enduring-word"]);
        let names = |apps: Vec<App>| apps.iter().map(|a| a.id).collect::<Vec<_>>();
        // Unknown and repeated ids are left out.
        let saved: Vec<String> =
            ["enduring-word", "no-such-app", "logos", "enduring-word"].map(String::from).into();
        assert_eq!(names(starred(&saved)), ["enduring-word", "logos"]);
        assert_eq!(
            names(menu_order(&saved)),
            [
                "enduring-word",
                "logos",
                "bible-app",
                "literal-word",
                "verse-notes",
                "blue-letter-bible",
                "step-bible",
                "desiring-god",
                "tgc-commentary",
                "chatgpt",
            ]
        );
        assert_eq!(menu_order(&[]), APPS.to_vec());
    }

    #[test]
    fn a_passage_is_one_chapter() {
        let john = BookId::from_code("JHN").unwrap();
        let refs = [VerseRef::new(john, 3, 18), VerseRef::new(john, 3, 16), VerseRef::new(john, 3, 17)];
        assert_eq!(Passage::from_refs(&refs), Some(Passage { book: john, chapter: 3, start: 16, end: 18 }));
        // Across chapters there is no passage, so the menu's Open in › is off.
        assert_eq!(Passage::from_refs(&[VerseRef::new(john, 3, 36), VerseRef::new(john, 4, 1)]), None);
        assert_eq!(Passage::from_refs(&[]), None);
    }
}
