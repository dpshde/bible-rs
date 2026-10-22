//! Template-variable context for route.bible adapter link expansion.
//!
//! Ported from `@route-bible/adapters` (`packages/route-bible-adapters/src/context.ts` and
//! `book-variants.ts`). Keeps Open in › offline: every `{{placeholder}}` the vendored
//! `adapters.links.json` uses is filled locally.

use std::collections::BTreeMap;

use crate::books::BOOKS;
use crate::open_in::Passage;

fn resolve_youversion(code: &str) -> (&'static str, &'static str) {
    match code {
        "BSB" => ("3034", "BSB"),
        "KJV" => ("1", "KJV"),
        "NKJV" => ("114", "NKJV"),
        "ESV" => ("59", "ESV"),
        "NIV" => ("111", "NIV"),
        "NLT" => ("116", "NLT"),
        "CSB" => ("1713", "CSB"),
        "NASB95" => ("100", "NASB1995"),
        "NASB1995" => ("100", "NASB1995"),
        "NRSV" => ("2015", "NRSV-CI"),
        "NRSV-CI" => ("2015", "NRSV-CI"),
        "NRSVUE" => ("3523", "NRSVUE"),
        "NET" => ("107", "NET"),
        "WEB" => ("206", "WEBUS"),
        "WEBUS" => ("206", "WEBUS"),
        _ => ("3034", "BSB"),
    }
}
/// Bolls.life translation slug
fn resolve_bolls_translation(code: &str) -> &'static str {
    match code {
        "BSB" => "ESV",
        "KJV" => "KJV",
        "NKJV" => "NKJV",
        "ESV" => "ESV",
        "NIV" => "NIV",
        "NLT" => "NLT",
        "CSB" => "CSB17",
        "NASB95" => "NASB",
        "NASB1995" => "NASB",
        "NRSV" => "NRSVCE",
        "NRSV-CI" => "NRSVCE",
        "NRSVUE" => "NRSVCE",
        "NET" => "NET",
        "WEB" => "WEB",
        "WEBUS" => "WEB",
        _ => "ESV",
    }
}
/// Type the Word translation
fn resolve_type_the_word_translation(code: &str) -> &'static str {
    match code {
        "KJV" => "nkjv",
        "NKJV" => "nkjv",
        "ESV" => "esv",
        "NIV" => "niv",
        "NLT" => "nlt",
        "CSB" => "csb",
        "NASB95" => "nasb",
        "NASB1995" => "nasb",
        "NRSV" => "esv",
        "NRSV-CI" => "esv",
        "NRSVUE" => "esv",
        "NET" => "esv",
        "WEB" => "esv",
        "WEBUS" => "esv",
        _ => "esv",
    }
}
/// STEP Bible translation
fn resolve_step_bible_translation(code: &str) -> &'static str {
    match code {
        "KJV" => "KJV",
        "NKJV" => "KJV",
        "ESV" => "ESV",
        "NIV" => "NIV",
        "NLT" => "ESV",
        "CSB" => "HCSB",
        "HCSB" => "HCSB",
        "NASB95" => "NASB",
        "NASB1995" => "NASB",
        "NRSV" => "NRSV",
        "NRSV-CI" => "NRSV",
        "NRSVUE" => "NRSV",
        "NET" => "ESV",
        "WEB" => "ESV",
        "WEBUS" => "ESV",
        _ => "ESV",
    }
}
/// Bible.is translation
fn resolve_bible_is_translation(code: &str) -> &'static str {
    match code {
        "KJV" => "engkjv",
        "NKJV" => "engnkjv",
        "ESV" => "engesv",
        "NIV" => "engniv",
        "NLT" => "engnlt",
        "CSB" => "engcsb",
        "NASB95" => "engnas",
        "NASB1995" => "engnas",
        "NRSV" => "engesv",
        "NRSV-CI" => "engesv",
        "NRSVUE" => "engesv",
        "NET" => "engesv",
        "WEB" => "engweb",
        "WEBUS" => "engweb",
        _ => "engesv",
    }
}
/// StudyLight translation (kept for parity with route.bible context.ts; unused by current templates).
#[allow(dead_code)]
fn resolve_study_light_translation(code: &str) -> &'static str {
    match code {
        "ESV" => "esv",
        "NLT" => "nlt",
        "NASB95" => "n95",
        "NASB1995" => "n95",
        _ => "kjv",
    }
}
/// studybible.info translation
fn resolve_study_bible_translation(code: &str) -> &'static str {
    match code {
        "KJV" => "KJV",
        "LEB" => "LEB",
        "BSB" => "BSB",
        _ => "YLT",
    }
}
/// Biblia.com translation
fn resolve_biblia_translation(code: &str) -> &'static str {
    match code {
        "KJV" => "kjv",
        "NIV" => "niv",
        "NASB95" => "nasb95",
        "NASB1995" => "nasb95",
        _ => "esv",
    }
}
fn enduring_word_override(code: &str) -> Option<&'static str> {
    match code {
        "PSA" => Some("psalm"),
        "SNG" => Some("song-of-solomon"),
        _ => None,
    }
}
fn blb_override(code: &str) -> Option<&'static str> {
    match code {
        "EZK" => Some("eze"),
        "JOL" => Some("joe"),
        _ => None,
    }
}
fn type_the_word_override(code: &str) -> Option<&'static str> {
    match code {
        "PSA" => Some("psalm"),
        _ => None,
    }
}
fn biblica_override(code: &str) -> Option<&'static str> {
    match code {
        "GEN" => Some("gen."),
        "LEV" => Some("lev."),
        "NUM" => Some("num."),
        "DEU" => Some("deut."),
        "1CH" => Some("1-chron."),
        "2CH" => Some("2-chron."),
        "NEH" => Some("neh."),
        "PSA" => Some("psalm"),
        "PRO" => Some("prov."),
        "ECC" => Some("eccles."),
        "JER" => Some("jer."),
        "EZK" => Some("ezek."),
        "HAB" => Some("hab."),
        "ZEC" => Some("zech."),
        "MAL" => Some("mal."),
        "MAT" => Some("matt."),
        "SNG" => Some("song-of-songs"),
        "1CO" => Some("1-cor."),
        "2CO" => Some("2-cor."),
        "PHP" => Some("phil."),
        "1TH" => Some("1-thes."),
        "2TH" => Some("2-thes."),
        "1TI" => Some("1-tim."),
        "2TI" => Some("2-tim."),
        "HEB" => Some("heb."),
        _ => None,
    }
}
fn catena_override(code: &str) -> Option<&'static str> {
    match code {
        "EXO" => Some("ex"),
        "LEV" => Some("lv"),
        "NUM" => Some("nm"),
        "DEU" => Some("dt"),
        "JOS" => Some("jos"),
        "JDG" => Some("jg"),
        "RUT" => Some("rt"),
        "1SA" => Some("1sam"),
        "2SA" => Some("2sam"),
        "1KI" => Some("1kgs"),
        "2KI" => Some("2kgs"),
        "1CH" => Some("1chron"),
        "2CH" => Some("2chron"),
        "EZR" => Some("ezr"),
        "NEH" => Some("neh"),
        "EST" => Some("est"),
        "JOB" => Some("jb"),
        "PSA" => Some("ps"),
        "PRO" => Some("prov"),
        "ECC" => Some("eccl"),
        "SNG" => Some("song"),
        "ISA" => Some("isa"),
        "JER" => Some("jer"),
        "LAM" => Some("lam"),
        "EZK" => Some("ez"),
        "DAN" => Some("dn"),
        "HOS" => Some("hos"),
        "JOL" => Some("jl"),
        "AMO" => Some("am"),
        "OBA" => Some("ob"),
        "JON" => Some("jon"),
        "MIC" => Some("mi"),
        "NAM" => Some("nah"),
        "HAB" => Some("hab"),
        "ZEP" => Some("zep"),
        "HAG" => Some("hag"),
        "ZEC" => Some("zec"),
        "MAL" => Some("mal"),
        "MAT" => Some("mt"),
        "MRK" => Some("mk"),
        "LUK" => Some("lk"),
        "JHN" => Some("jn"),
        "ACT" => Some("acts"),
        "ROM" => Some("rom"),
        "1CO" => Some("1cor"),
        "2CO" => Some("2cor"),
        "GAL" => Some("gal"),
        "EPH" => Some("eph"),
        "PHP" => Some("phil"),
        "COL" => Some("col"),
        "1TH" => Some("1thess"),
        "2TH" => Some("2thess"),
        "1TI" => Some("1tim"),
        "2TI" => Some("2tim"),
        "TIT" => Some("ti"),
        "PHM" => Some("phm"),
        "HEB" => Some("heb"),
        "JAS" => Some("jas"),
        "1PE" => Some("1pet"),
        "2PE" => Some("2pet"),
        "1JN" => Some("1jn"),
        "2JN" => Some("2jn"),
        "3JN" => Some("3jn"),
        "JUD" => Some("jude"),
        "REV" => Some("rev"),
        _ => None,
    }
}
fn step_bible_override(code: &str) -> Option<&'static str> {
    match code {
        "EXO" => Some("Exod"),
        "LEV" => Some("Lev"),
        "NUM" => Some("Num"),
        "DEU" => Some("Deut"),
        "JOS" => Some("Josh"),
        "JDG" => Some("Judg"),
        "RUT" => Some("Ruth"),
        "1SA" => Some("1Sam"),
        "2SA" => Some("2Sam"),
        "1KI" => Some("1Kgs"),
        "2KI" => Some("2Kgs"),
        "1CH" => Some("1Chron"),
        "2CH" => Some("2Chron"),
        "EZR" => Some("Ezra"),
        "NEH" => Some("Neh"),
        "EST" => Some("Esth"),
        "JOB" => Some("Job"),
        "PSA" => Some("Psalm"),
        "PRO" => Some("Prov"),
        "ECC" => Some("Eccl"),
        "SNG" => Some("Song"),
        "ISA" => Some("Isa"),
        "JER" => Some("Jer"),
        "LAM" => Some("Lam"),
        "EZK" => Some("Ezek"),
        "DAN" => Some("Dan"),
        "HOS" => Some("Hos"),
        "JOL" => Some("Joel"),
        "AMO" => Some("Amos"),
        "OBA" => Some("Obad"),
        "JON" => Some("Jon"),
        "MIC" => Some("Mic"),
        "NAM" => Some("Nah"),
        "HAB" => Some("Hab"),
        "ZEP" => Some("Zeph"),
        "HAG" => Some("Hag"),
        "ZEC" => Some("Zech"),
        "MAL" => Some("Mal"),
        "MAT" => Some("Matt"),
        "MRK" => Some("Mark"),
        "LUK" => Some("Luke"),
        "JHN" => Some("John"),
        "ACT" => Some("Acts"),
        "ROM" => Some("Rom"),
        "1CO" => Some("1Cor"),
        "2CO" => Some("2Cor"),
        "GAL" => Some("Gal"),
        "EPH" => Some("Eph"),
        "PHP" => Some("Phil"),
        "COL" => Some("Col"),
        "1TH" => Some("1Thess"),
        "2TH" => Some("2Thess"),
        "1TI" => Some("1Tim"),
        "2TI" => Some("2Tim"),
        "TIT" => Some("Tit"),
        "PHM" => Some("Phlm"),
        "HEB" => Some("Heb"),
        "JAS" => Some("Jas"),
        "1PE" => Some("1Pet"),
        "2PE" => Some("2Pet"),
        "1JN" => Some("1John"),
        "2JN" => Some("2John"),
        "3JN" => Some("3John"),
        "JUD" => Some("Jude"),
        "REV" => Some("Rev"),
        _ => None,
    }
}
fn bible_project_guide(code: &str) -> Option<&'static str> {
    match code {
        "EXO" => Some("book-of-exodus"),
        "LEV" => Some("book-of-leviticus"),
        "NUM" => Some("book-of-numbers"),
        "DEU" => Some("book-of-deuteronomy"),
        "JOS" => Some("book-of-joshua"),
        "JDG" => Some("book-of-judges"),
        "RUT" => Some("book-of-ruth"),
        "NEH" => Some("book-of-ezra-nehemiah"),
        "EST" => Some("book-of-esther"),
        "JOB" => Some("book-of-job"),
        "PSA" => Some("book-of-psalms"),
        "PRO" => Some("book-of-proverbs"),
        "ECC" => Some("book-of-ecclesiastes"),
        "SNG" => Some("book-of-song-of-songs"),
        "ISA" => Some("book-of-isaiah"),
        "JER" => Some("book-of-jeremiah"),
        "LAM" => Some("book-of-lamentations"),
        "EZK" => Some("book-of-ezekiel"),
        "DAN" => Some("book-of-daniel"),
        "HOS" => Some("book-of-hosea"),
        "JOL" => Some("book-of-joel"),
        "AMO" => Some("book-of-amos"),
        "OBA" => Some("book-of-obadiah"),
        "JON" => Some("book-of-jonah"),
        "MIC" => Some("book-of-micah"),
        "NAM" => Some("book-of-nahum"),
        "HAB" => Some("book-of-habakkuk"),
        "ZEP" => Some("book-of-zephaniah"),
        "HAG" => Some("book-of-haggai"),
        "ZEC" => Some("book-of-zechariah"),
        "MAL" => Some("book-of-malachi"),
        "MAT" => Some("book-of-matthew"),
        "MRK" => Some("book-of-mark"),
        "LUK" => Some("book-of-luke"),
        "JHN" => Some("book-of-john"),
        "ACT" => Some("book-of-acts"),
        "ROM" => Some("book-of-romans"),
        "1CO" => Some("book-of-1-corinthians"),
        "2CO" => Some("book-of-2-corinthians"),
        "GAL" => Some("book-of-galatians"),
        "EPH" => Some("book-of-ephesians"),
        "PHP" => Some("book-of-philippians"),
        "COL" => Some("book-of-colossians"),
        "1TH" => Some("book-of-1-thessalonians"),
        "2TH" => Some("book-of-2-thessalonians"),
        "1TI" => Some("book-of-1-timothy"),
        "2TI" => Some("book-of-2-timothy"),
        "TIT" => Some("book-of-titus"),
        "PHM" => Some("book-of-philemon"),
        "HEB" => Some("book-of-hebrews"),
        "JAS" => Some("book-of-james"),
        "1PE" => Some("book-of-1-peter"),
        "2PE" => Some("book-of-2-peter"),
        "REV" => Some("book-of-revelation"),
        _ => None,
    }
}
fn open_bible_geo_slug(code: &str) -> Option<&'static str> {
    match code {
        "EXO" => Some("exod"),
        "LEV" => Some("lev"),
        "NUM" => Some("num"),
        "DEU" => Some("deut"),
        "JOS" => Some("josh"),
        "JDG" => Some("judg"),
        "RUT" => Some("ruth"),
        "1SA" => Some("1sam"),
        "2SA" => Some("2sam"),
        "1KI" => Some("1kgs"),
        "2KI" => Some("2kgs"),
        "1CH" => Some("1chr"),
        "2CH" => Some("2chr"),
        "EZR" => Some("ezra"),
        "NEH" => Some("neh"),
        "EST" => Some("esth"),
        "JOB" => Some("job"),
        "PSA" => Some("ps"),
        "PRO" => Some("prov"),
        "ECC" => Some("eccl"),
        "SNG" => Some("song"),
        "ISA" => Some("isa"),
        "JER" => Some("jer"),
        "LAM" => Some("lam"),
        "EZK" => Some("ezek"),
        "DAN" => Some("dan"),
        "HOS" => Some("hos"),
        "JOL" => Some("joel"),
        "AMO" => Some("amos"),
        "OBA" => Some("obad"),
        "JON" => Some("jonah"),
        "MIC" => Some("mic"),
        "NAM" => Some("nah"),
        "HAB" => Some("hab"),
        "ZEP" => Some("zeph"),
        "HAG" => Some("hag"),
        "ZEC" => Some("zech"),
        "MAL" => Some("mal"),
        "MAT" => Some("matt"),
        "MRK" => Some("mark"),
        "LUK" => Some("luke"),
        "JHN" => Some("john"),
        "ACT" => Some("acts"),
        "ROM" => Some("rom"),
        "1CO" => Some("1cor"),
        "2CO" => Some("2cor"),
        "GAL" => Some("gal"),
        "EPH" => Some("eph"),
        "PHP" => Some("phil"),
        "COL" => Some("col"),
        "1TH" => Some("1thess"),
        "2TH" => Some("2thess"),
        "1TI" => Some("1tim"),
        "2TI" => Some("2tim"),
        "TIT" => Some("titus"),
        "PHM" => Some("philem"),
        "HEB" => Some("heb"),
        "JAS" => Some("james"),
        "1PE" => Some("1pet"),
        "2PE" => Some("2pet"),
        "1JN" => Some("1john"),
        "2JN" => Some("2john"),
        "3JN" => Some("3john"),
        "JUD" => Some("jude"),
        "REV" => Some("rev"),
        _ => None,
    }
}
fn open_bible_geo_unsupported(code: &str) -> bool {
    matches!(code, "PHM" | "JAS" | "1JN" | "2JN" | "3JN")
}
fn open_bible_geo_chapters(slug: &str) -> &'static [u8] {
    match slug {
        "1cor" => &[1, 15, 16],
        "1kgs" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22],
        "1pet" => &[1, 2, 5],
        "1sam" => &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 21, 22, 23, 24, 25, 26, 27,
            28, 29, 30, 31,
        ],
        "1thess" => &[1, 2, 3, 4],
        "1tim" => &[1],
        "2chr" => &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
            27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
        ],
        "2cor" => &[1, 2, 7, 8, 9, 10, 11],
        "2kgs" => {
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
        }
        "2pet" => &[2],
        "2sam" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24],
        "2thess" => &[1],
        "2tim" => &[1, 3, 4],
        "acts" => &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
            27, 28,
        ],
        "amos" => &[1, 2, 3, 4, 5, 6, 7, 8, 9],
        "col" => &[1, 2, 4],
        "dan" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        "deut" => &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 20, 23, 24, 25, 26, 27, 28, 29, 30,
            31, 32, 33, 34,
        ],
        "eccl" => &[1, 2],
        "eph" => &[1],
        "esth" => &[1, 2, 3, 4, 8, 9, 10],
        "exod" => &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24, 26, 28, 29,
            31, 32, 33, 34, 35, 39, 40,
        ],
        "ezek" => &[
            1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
            29, 30, 31, 32, 33, 35, 36, 38, 39, 40, 41, 42, 43, 44, 45, 47, 48,
        ],
        "ezra" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        "gal" => &[1, 2, 4],
        "gen" => &[
            2, 3, 4, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 31,
            32, 33, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
        ],
        "hab" => &[1, 2, 3],
        "hag" => &[2],
        "heb" => &[3, 7, 8, 9, 10, 11, 12, 13],
        "hos" => &[1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
        "isa" => &[
            1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27, 28,
            29, 30, 31, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 51, 52, 54, 56,
            59, 60, 61, 62, 63, 64, 65, 66,
        ],
        "jer" => &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
            27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 46, 47, 48, 49, 50, 51,
            52,
        ],
        "job" => &[1, 2, 6, 11, 20, 22, 26, 28, 40, 42],
        "joel" => &[2, 3],
        "john" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 18, 19, 21],
        "jonah" => &[1, 3, 4],
        "josh" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24],
        "jude" => &[1],
        "judg" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
        "lam" => &[1, 2, 3, 4, 5],
        "lev" => &[6, 7, 10, 11, 14, 16, 18, 19, 22, 23, 25, 26, 27],
        "luke" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 17, 18, 19, 21, 22, 23, 24],
        "mal" => &[1, 2, 3, 4],
        "mark" => &[1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16],
        "matt" => &[1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 21, 23, 24, 26, 27, 28],
        "mic" => &[1, 2, 3, 4, 5, 6, 7],
        "nah" => &[1, 2, 3],
        "neh" => &[1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13],
        "num" => &[
            1, 3, 8, 9, 10, 11, 12, 13, 14, 15, 16, 20, 21, 22, 23, 24, 25, 26, 27, 28, 31, 32, 33, 34, 35,
            36,
        ],
        "obad" => &[1],
        "phil" => &[1, 4],
        "prov" => &[7],
        "ps" => &[
            2, 9, 14, 20, 22, 28, 29, 37, 42, 45, 48, 50, 51, 53, 56, 60, 65, 66, 68, 69, 72, 74, 76, 77, 78,
            79, 80, 81, 83, 84, 87, 89, 92, 95, 97, 99, 102, 104, 105, 106, 108, 110, 114, 116, 120, 121,
            122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 146, 147, 149,
        ],
        "rev" => &[1, 2, 3, 9, 11, 14, 16, 17, 18, 20, 21],
        "rom" => &[1, 9, 11, 15, 16],
        "ruth" => &[1, 2, 4],
        "song" => &[1, 2, 3, 4, 5, 6, 7, 8],
        "titus" => &[1, 3],
        "zech" => &[1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
        "zeph" => &[1, 2, 3],
        _ => &[],
    }
}

fn normalize_translation(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "BSB".into();
    }
    let normalized: String = trimmed.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if normalized.is_empty() { "BSB".into() } else { normalized.to_ascii_uppercase() }
}

fn encode_uri_component(s: &str) -> String {
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

fn to_slug(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let stripped: String =
        lower.chars().filter(|c| !matches!(c, '\'' | '"' | '.' | ',' | '(' | ')')).collect();
    let mut out = String::new();
    let mut prev_dash = false;
    for c in stripped.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn to_snake_slug(value: &str) -> String {
    to_slug(value).replace('-', "_")
}

fn to_compact_title(value: &str) -> String {
    value.chars().filter(|c| !matches!(c, '\'' | '"' | '.' | ',' | '(' | ')' | ' ')).collect()
}

fn to_compact_pascal(value: &str) -> String {
    value
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase(),
            }
        })
        .collect()
}

fn book_name(code: &str) -> String {
    BOOKS.iter().find(|b| b.code == code).map(|b| b.name.to_string()).unwrap_or_else(|| code.to_string())
}

fn service_book_variant(service: &str, code: &str) -> String {
    let name = book_name(code);
    match service {
        "canonical" => code.to_string(),
        "slug" => to_slug(&name),
        "bible-gateway" => name,
        "bible-hub" => to_snake_slug(&name),
        "biblica" => biblica_override(code).map(|s| s.to_string()).unwrap_or_else(|| to_slug(&name)),
        "enduring-word" => {
            enduring_word_override(code).map(|s| s.to_string()).unwrap_or_else(|| to_slug(&name))
        }
        "blue-letter-bible" => {
            blb_override(code).map(|s| s.to_string()).unwrap_or_else(|| code.to_ascii_lowercase())
        }
        "type-the-word" => {
            type_the_word_override(code).map(|s| s.to_string()).unwrap_or_else(|| to_snake_slug(&name))
        }
        "catena" => {
            catena_override(code).map(|s| s.to_string()).unwrap_or_else(|| to_slug(&name).replace('-', ""))
        }
        "step-bible" => {
            step_bible_override(code).map(|s| s.to_string()).unwrap_or_else(|| to_compact_title(&name))
        }
        "logos-ref-ly" => to_compact_pascal(&name),
        "sermon-audio" | "bible-is" => code.to_ascii_lowercase(),
        _ => code.to_string(),
    }
}

fn logos_book_number(code: &str) -> u32 {
    let idx = BOOKS.iter().position(|b| b.code == code).unwrap_or(0) as u32;
    let n = idx + 1;
    if n >= 40 { n + 21 } else { n }
}

fn study_light_original_language(code: &str) -> &'static str {
    let idx = BOOKS.iter().position(|b| b.code == code).unwrap_or(0);
    if idx >= 39 { "greek" } else { "hebrew" }
}

fn range_type(p: Passage) -> &'static str {
    if !p.has_verse() {
        "chapter"
    } else if p.verse_start == p.verse_end {
        "single"
    } else {
        "same_chapter"
    }
}

fn format_bible_gateway(p: Passage) -> String {
    let name = book_name(p.book.info().code);
    if !p.has_verse() {
        format!("{name} {}", p.chapter)
    } else if p.verse_start == p.verse_end {
        format!("{name} {}:{}", p.chapter, p.verse_start)
    } else {
        format!("{name} {}:{}-{}", p.chapter, p.verse_start, p.verse_end)
    }
}

fn format_literal_word(p: Passage) -> String {
    let name = book_name(p.book.info().code);
    if !p.has_verse() {
        format!("{name} {}", p.chapter)
    } else if p.verse_start == p.verse_end {
        format!("{name} {} {}", p.chapter, p.verse_start)
    } else {
        format!("{name} {} {}-{}", p.chapter, p.verse_start, p.verse_end)
    }
}

fn format_logos_ref_ly(p: Passage) -> String {
    let code = p.book.info().code;
    let book = if code == "PSA" { "Psalm".to_string() } else { service_book_variant("logos-ref-ly", code) };
    let ch = p.chapter;
    if !p.has_verse() {
        format!("{book}{ch}")
    } else if p.verse_start == p.verse_end {
        format!("{book}{ch}.{}", p.verse_start)
    } else {
        format!("{book}{ch}.{}-{}", p.verse_start, p.verse_end)
    }
}

fn format_logos_data_reference(p: Passage, translation: &str) -> String {
    let t = translation.to_ascii_lowercase();
    let t = if t.is_empty() { "esv".to_string() } else { t };
    let book_number = logos_book_number(p.book.info().code);
    let chapter = p.chapter;
    let start = if p.has_verse() { p.verse_start } else { 1 };
    if p.has_verse() && p.verse_end > p.verse_start {
        format!("bible+{t}.{book_number}.{chapter}.{start}-{}", p.verse_end)
    } else {
        format!("bible+{t}.{book_number}.{chapter}.{start}")
    }
}

fn verse_range_fragment(p: Passage) -> String {
    if !p.has_verse() {
        return String::new();
    }
    if p.verse_start == p.verse_end {
        format!("#{}", p.verse_start)
    } else {
        format!("#{}-{}", p.verse_start, p.verse_end)
    }
}

fn bolls_verse_path(p: Passage) -> String {
    if !p.has_verse() {
        return "/".into();
    }
    if p.verse_end > p.verse_start {
        format!("/{}-{}/", p.verse_start, p.verse_end)
    } else {
        format!("/{}/", p.verse_start)
    }
}

fn type_the_word_path(p: Passage) -> String {
    let slug = service_book_variant("type-the-word", p.book.info().code);
    let base = format!("{slug}_{}", p.chapter);
    if !p.has_verse() {
        base
    } else if p.verse_start == p.verse_end {
        format!("{base}:{}", p.verse_start)
    } else {
        format!("{base}:{}-{}", p.verse_start, p.verse_end)
    }
}

fn path_book_chapter_verse(service: &str, p: Passage, sep: &str) -> String {
    let book = service_book_variant(service, p.book.info().code);
    let chapter = p.chapter.to_string();
    if !p.has_verse() {
        format!("{book}{sep}{chapter}")
    } else {
        format!("{book}{sep}{chapter}{sep}{}", p.verse_start)
    }
}

fn bible_hub_path(p: Passage) -> String {
    let book = service_book_variant("bible-hub", p.book.info().code);
    let chapter = p.chapter;
    if p.has_verse() && p.verse_start == p.verse_end {
        format!("{book}/{chapter}-{}.htm", p.verse_start)
    } else {
        format!("{book}/{chapter}.htm")
    }
}

fn bible_study_tools_path(p: Passage) -> String {
    let book = service_book_variant("slug", p.book.info().code);
    let chapter = p.chapter;
    if p.has_verse() && p.verse_start == p.verse_end {
        format!("{book}/{chapter}-{}.html", p.verse_start)
    } else {
        format!("{book}/{chapter}.html")
    }
}

fn catena_path(p: Passage) -> String {
    path_book_chapter_verse("catena", p, "/")
}

fn sermon_audio_path(p: Passage) -> String {
    let book = service_book_variant("sermon-audio", p.book.info().code);
    let chapter = p.chapter;
    if p.has_verse() && p.verse_start == p.verse_end {
        format!("{book}/{chapter}/{}", p.verse_start)
    } else {
        format!("{book}/{chapter}")
    }
}

fn bible_is_path(p: Passage) -> String {
    let book = service_book_variant("bible-is", p.book.info().code);
    let chapter = p.chapter;
    if p.has_verse() && p.verse_start == p.verse_end {
        format!("{book}/{chapter}/{}", p.verse_start)
    } else {
        format!("{book}/{chapter}")
    }
}

fn biblica_path(p: Passage) -> String {
    let book = service_book_variant("biblica", p.book.info().code);
    let chapter = p.chapter;
    if p.has_verse() && p.verse_start == p.verse_end {
        format!("{book}/{chapter}/{}", p.verse_start)
    } else {
        format!("{book}/{chapter}")
    }
}

fn bible_ref_path(p: Passage) -> String {
    let book_name = service_book_variant("bible-gateway", p.book.info().code);
    let book_segment: String = book_name
        .chars()
        .map(|c| if matches!(c, '\'' | '"' | '.' | ',' | '(' | ')') { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let verse_book = if p.book.info().code == "PSA" { "Psalm".to_string() } else { book_segment.clone() };
    let chapter = p.chapter;
    if range_type(p) == "single" {
        let verse = if p.has_verse() { p.verse_start } else { 1 };
        format!("{book_segment}/{chapter}/{verse_book}-{chapter}-{verse}.html")
    } else {
        format!("{book_segment}/{chapter}/{book_segment}-chapter-{chapter}.html")
    }
}

fn step_bible_query(p: Passage, translation: &str) -> String {
    let t = resolve_step_bible_translation(translation);
    let book = service_book_variant("step-bible", p.book.info().code);
    match range_type(p) {
        "chapter" => format!("version={t}@reference={book}.{}", p.chapter),
        "single" => format!("version={t}@reference={book}.{}.{}", p.chapter, p.verse_start),
        _ => format!("version={t}@reference={book}.{}.{}-{}", p.chapter, p.verse_start, p.verse_end),
    }
}

fn bible_project_path(p: Passage) -> String {
    if let Some(guide) = bible_project_guide(p.book.info().code) {
        format!("guides/{guide}/")
    } else {
        let q = encode_uri_component(&format_bible_gateway(p));
        format!("search/?q={q}")
    }
}

fn open_bible_geo_path(p: Passage) -> String {
    let code = p.book.info().code;
    if open_bible_geo_unsupported(code) {
        return String::new();
    }
    let Some(slug) = open_bible_geo_slug(code) else {
        return String::new();
    };
    let chapters = open_bible_geo_chapters(slug);
    if chapters.contains(&p.chapter) { format!("{slug}.{}", p.chapter) } else { slug.to_string() }
}

fn route_bible_url(p: Passage, translation: &str) -> String {
    format!("https://route.bible{}?v={translation}", p.route_path())
}

fn build_ai_prompt_body(display: &str, translation: &str) -> String {
    // Mirrors route.bible's Kg(): reference link only — no local verse text in the prompt.
    let link = format!(
        "[{display}](https://route.bible/?q={}&utm_source=chatgpt&utm_medium=prompt)",
        encode_uri_component(display)
    );
    format!(
        "I have a question about a Bible passage. Use your knowledge of the text to answer.\n\n\
RULE: Every Scripture reference you write must be a Markdown link to route.bible using OSIS notation in the URL.\n\n\
Passage in question:\n{link}\n\nTranslation: {translation}"
    )
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

/// Percent-encoded raw verse text for translate URL `text=` params.
pub fn encoded_raw_text(verse_text: &str) -> String {
    encode_uri_component(&truncate_for_url(verse_text.trim()))
}

fn obsidian_note_name(display: &str, translation: &str) -> String {
    let sanitized = display.replace(':', ".").replace(['<', '>', '"', '/', '\\', '|', '?', '*'], " ");
    let sanitized = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{sanitized} ({translation})")
}

fn obsidian_note_content(p: Passage, display: &str, translation: &str, route_url: &str) -> String {
    let book = book_name(p.book.info().code);
    let slug = service_book_variant("slug", p.book.info().code);
    let captured = "2026-01-01"; // deterministic; route.bible uses today's date
    let bridges = format!("  - \"[[{book} {}]]\"", p.chapter);
    format!(
        "---\ntranslation: \"{translation}\"\nsource: \"[route.bible]({route_url})\"\ndate: \"{captured}\"\n\
reference: \"{display}\"\nosis: \"{}\"\nbook: \"{book}\"\nbridges:\n{bridges}\n  - \"[[{book}]]\"\n\
tags:\n  - \"route-bible\"\n  - \"scripture\"\n  - \"book/{slug}\"\n  - \"translation/{}\"\n---\n\n\
# {display}\n\nSource: [route.bible]({route_url})\n\n## Notes\n",
        p.canonical(),
        translation.to_ascii_lowercase()
    )
}

fn obsidian_reading_log_base() -> &'static str {
    "filters:\n  and:\n    - file.hasTag(\"route-bible\")\nviews:\n  - type: table\n    name: Log\n    order:\n      - file.name\n      - translation\n      - date\n      - source\n      - file.mtime\n      - book\n      - reference\n      - tags"
}

/// Build substitution values for every `{{placeholder}}` used by the vendored links snapshot.
/// Values that belong in query strings are already `encodeURIComponent`-encoded.
///
/// `verse_text` is the open translation's plain verse text (spaces between verses, no numbers).
pub fn template_replacements(
    passage: Passage,
    translation: &str,
    verse_text: &str,
) -> BTreeMap<&'static str, String> {
    let translation = normalize_translation(translation);
    let code = passage.book.info().code;
    let (yv_id, yv_code) = resolve_youversion(&translation);
    let display = format_bible_gateway(passage);
    let route_url = route_bible_url(passage, &translation);
    let action_title = format!("{display} ({translation})");
    let text_for_url = truncate_for_url(verse_text.trim());
    let action_body = action_body_with_text(&text_for_url, &display, &translation);
    let book_slug = service_book_variant("slug", code);
    let book_code = service_book_variant("blue-letter-bible", code);
    let book_number = (BOOKS.iter().position(|b| b.code == code).unwrap_or(0) + 1).to_string();
    let chapter = passage.chapter.to_string();
    let verse = if passage.has_verse() { passage.verse_start.to_string() } else { "1".into() };
    let enduring = service_book_variant("enduring-word", code);
    let chapter_path = format!("{enduring}-{chapter}");
    let geo = open_bible_geo_path(passage);
    let ai = build_ai_prompt_body(&display, &translation);
    let todoist = format!("{} ({translation})", display.replace(':', "."));
    let note_name = obsidian_note_name(&display, &translation);
    let note_content = obsidian_note_content(passage, &display, &translation, &route_url);

    let mut m = BTreeMap::new();
    // Encoded (query-safe) values — matching route.bible's buildAdapterTemplateReplacements.
    m.insert("passage", encode_uri_component(&passage.canonical()));
    m.insert("passageLower", encode_uri_component(&passage.canonical().to_ascii_lowercase()));
    m.insert("displayPassage", encode_uri_component(&display));
    m.insert("translation", encode_uri_component(&translation));
    m.insert("translationLower", encode_uri_component(&translation.to_ascii_lowercase()));
    m.insert("routeUrl", encode_uri_component(&route_url));
    m.insert("shareText", encode_uri_component(&format!("{display} {route_url}")));
    m.insert("actionTitle", encode_uri_component(&action_title));
    m.insert("actionBody", encode_uri_component(&action_body));
    // Raw verse text only (for callers / future templates); translate expansion overrides actionBody.
    m.insert("passageText", encode_uri_component(&text_for_url));
    m.insert("aiPromptBody", encode_uri_component(&ai));
    m.insert("todoistTaskTitle", encode_uri_component(&todoist));
    m.insert("youversionVersionId", encode_uri_component(yv_id));
    m.insert("youversionTranslation", encode_uri_component(yv_code));
    m.insert("bollsTranslation", encode_uri_component(resolve_bolls_translation(&translation)));
    m.insert("typeTheWordTranslation", encode_uri_component(resolve_type_the_word_translation(&translation)));
    m.insert("studyBibleTranslation", encode_uri_component(resolve_study_bible_translation(&translation)));
    m.insert("studyLightOriginalLanguage", encode_uri_component(study_light_original_language(code)));
    m.insert("bibleIsTranslation", encode_uri_component(resolve_bible_is_translation(&translation)));
    m.insert("bibliaTranslation", encode_uri_component(resolve_biblia_translation(&translation)));
    m.insert("bookSlug", encode_uri_component(&book_slug));
    m.insert("bookCode", encode_uri_component(&book_code));
    m.insert("bookNumber", encode_uri_component(&book_number));
    m.insert("chapter", encode_uri_component(&chapter));
    m.insert("verse", encode_uri_component(&verse));
    m.insert("chapterPath", encode_uri_component(&chapter_path));
    m.insert("logosDataReference", encode_uri_component(&format_logos_data_reference(passage, "esv")));
    m.insert("logosRefLyReference", encode_uri_component(&format_logos_ref_ly(passage)));
    m.insert("literalWordQuery", encode_uri_component(&format_literal_word(passage)));
    m.insert("stepBibleQuery", encode_uri_component(&step_bible_query(passage, &translation)));
    m.insert("obsidianNoteName", encode_uri_component(&note_name));
    m.insert("obsidianNoteContent", encode_uri_component(&note_content));
    m.insert("obsidianReadingLogBaseContent", encode_uri_component(obsidian_reading_log_base()));
    // Path segments — left unencoded (already URL-safe).
    m.insert("typeTheWordPath", type_the_word_path(passage));
    m.insert("sermonAudioPath", sermon_audio_path(passage));
    m.insert("bibleIsPath", bible_is_path(passage));
    m.insert("biblicaPath", biblica_path(passage));
    m.insert("verseRange", verse_range_fragment(passage));
    m.insert("versePath", bolls_verse_path(passage));
    m.insert("bibleRefPath", bible_ref_path(passage));
    m.insert("bibleHubPath", bible_hub_path(passage));
    m.insert("bibleStudyToolsPath", bible_study_tools_path(passage));
    m.insert("catenaPath", catena_path(passage));
    m.insert("bibleProjectPath", bible_project_path(passage));
    m.insert("openBibleGeoPath", geo);
    m
}
