//! Which translations a destination actually serves, for adapters whose links name one.
//!
//! route.bible fills `{{translation}}` / `{{translationLower}}` with whatever translation is open,
//! so a site without it answers with a 404 (WeDevote has no BSB, for one). When the open
//! translation isn't in an adapter's list, Open in › drops the translation part of the link and the
//! site opens in its own default version instead.
//!
//! Lists were checked by requesting each site with each code (2026-10-22); an adapter not listed
//! here keeps its link unchanged. Codes are route.bible-normalized (uppercase, alphanumerics).

/// One adapter's served translations and the template piece that names the translation.
pub struct TranslationSupport {
    pub adapter: &'static str,
    /// Template text removed when the translation isn't served.
    pub segment: &'static str,
    pub supported: &'static [&'static str],
}

pub static TRANSLATION_SUPPORT: &[TranslationSupport] = &[
    // wd.bible/en/bible/jon.2.bsb → /en/404; wd.bible/en/bible/jon.2 opens its default (NIV).
    TranslationSupport {
        adapter: "wedevote",
        segment: ".{{translationLower}}",
        supported: &[
            "AMP", "ASV", "CSB", "ESV", "KJV", "LSB", "MSG", "NASB", "NET", "NIRV", "NIV", "NKJV", "NLT",
            "WEB",
        ],
    },
    // blueletterbible.org/bsb/jon/2/3/ → 404; /jon/2/3/ opens its default (KJV).
    TranslationSupport {
        adapter: "blue-letter-bible",
        segment: "{{translationLower}}/",
        supported: &[
            "AMP", "ASV", "CSB", "DBY", "ESV", "KJV", "LSB", "NASB", "NASB95", "NET", "NIV", "NKJV", "NLT",
            "RSV", "WEB", "YLT",
        ],
    },
    // biblegateway.com with version=BSB shows no text; without version it uses the reader's default.
    TranslationSupport {
        adapter: "bible-gateway",
        segment: "&version={{translation}}",
        supported: &[
            "AMP", "ASV", "CSB", "DARBY", "ESV", "GNT", "HCSB", "KJV", "LEB", "LSB", "MSG", "NASB",
            "NASB1995", "NET", "NIRV", "NIV", "NKJV", "NLT", "NRSV", "NRSVUE", "RSV", "WEB", "YLT",
        ],
    },
];

/// The template to expand for `adapter_id` with `translation` (route.bible-normalized): the
/// original, or the original without its translation part when the site doesn't serve it.
pub fn template_for<'a>(adapter_id: &str, template: &'a str, translation: &str) -> std::borrow::Cow<'a, str> {
    match TRANSLATION_SUPPORT.iter().find(|s| s.adapter == adapter_id) {
        Some(s) if template.contains(s.segment) && !s.supported.contains(&translation) => {
            std::borrow::Cow::Owned(template.replacen(s.segment, "", 1))
        }
        _ => std::borrow::Cow::Borrowed(template),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_translation_drops_the_segment() {
        let wd = "https://wd.bible/en/bible/{{bookCode}}.{{chapter}}.{{translationLower}}{{verseRange}}";
        assert_eq!(
            template_for("wedevote", wd, "BSB"),
            "https://wd.bible/en/bible/{{bookCode}}.{{chapter}}{{verseRange}}"
        );
        assert_eq!(template_for("wedevote", wd, "ESV"), wd);
        // Adapters without data keep their link.
        assert_eq!(template_for("inksignal", wd, "BSB"), wd);
    }

    #[test]
    fn lists_are_normalized_codes() {
        for s in TRANSLATION_SUPPORT {
            for t in s.supported {
                assert_eq!(*t, crate::open_in_context::normalize_translation(t), "{}: {t}", s.adapter);
            }
        }
    }
}
