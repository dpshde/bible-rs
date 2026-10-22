// Generates bible-rs's route.bible parity data from route.bible's own TypeScript:
//   crates/core/src/open_in_tables.rs                     (book variants, translation maps, geo chapters)
//   crates/core/tests/fixtures/route-bible-parity.json    (template values + rendered URLs, a sample)
// With ROUTE_BIBLE_PARITY_FULL=1 it also writes a full sweep to target/route-bible-parity-full.json.
// Run from a route.bible checkout (needs its node_modules for esbuild):
//   scripts/route-bible-parity/run <route.bible checkout>
import { mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { OSIS_BOOK_CODES, OSIS_BOOK_NAMES } from "rb/packages/grab-bcv/src/index";
import { parsePassage } from "rb/packages/grab-bcv/src/parser";
import { getServiceBookVariant, getBibleProjectGuideSlug } from "rb/packages/route-bible-adapters/src/book-variants";
import { buildAdapterTemplateContext, buildVerseQueryText } from "rb/packages/route-bible-adapters/src/context";
import { buildAdapterTemplateReplacements, renderAdapterTemplate } from "rb/packages/route-bible-adapters/src/registry";

const out = process.env.BIBLE_RS_DIR!;
const rbDir = process.env.ROUTE_BIBLE_DIR!;

// The translation maps and geo chapter lists are module-private in context.ts; lift the object
// literals out of the source so the tables can't drift from it.
function constObject(name: string): Record<string, any> {
  const src = readFileSync(`${rbDir}/packages/route-bible-adapters/src/context.ts`, "utf8");
  const start = src.indexOf(`const ${name}`);
  if (start < 0) throw new Error(`context.ts: ${name} not found`);
  const open = src.indexOf("= {", start) + 2;
  const close = src.indexOf("\n};", open);
  return new Function(`return (${src.slice(open, close + 2)});`)();
}

function mapFn(fnName: string, constName: string, doc: string): string {
  const m = constObject(constName);
  const arms = Object.entries(m).map(([k, v]) => `        ${JSON.stringify(k)} => Some(${JSON.stringify(v)}),`);
  return `\n/// ${doc}\n#[rustfmt::skip]\npub fn ${fnName}(code: &str) -> Option<&'static str> {\n    match code {\n${arms.join("\n")}\n        _ => None,\n    }\n}\n`;
}

function tables(): string {
  const yv = constObject("YOUVERSION_VERSION_MAP");
  const yvArms = Object.entries(yv).map(([k, v]: any) => `        ${JSON.stringify(k)} => Some((${JSON.stringify(v.versionId)}, ${JSON.stringify(v.translationCode)})),`);
  const geo = constObject("OPEN_BIBLE_GEO_CHAPTERS_WITH_PLACES");
  const geoArms = Object.entries(geo).map(([k, v]: any) => `        ${JSON.stringify(k)} => &[${v.join(", ")}],`);
  return [
    `\n/// YouVersion (version id, translation code) by route.bible translation code.\n#[rustfmt::skip]\npub fn youversion(code: &str) -> Option<(&'static str, &'static str)> {\n    match code {\n${yvArms.join("\n")}\n        _ => None,\n    }\n}\n`,
    mapFn("bolls", "BOLLS_TRANSLATION_MAP", "bolls.life translation."),
    mapFn("simply_my_bible", "SIMPLY_MY_BIBLE_TRANSLATION_MAP", "simplymybible.com translation."),
    mapFn("type_the_word", "TYPE_THE_WORD_TRANSLATION_MAP", "typetheword.site translation."),
    mapFn("step_bible", "STEP_BIBLE_TRANSLATION_MAP", "STEP Bible translation."),
    mapFn("bible_is", "BIBLE_IS_TRANSLATION_MAP", "Bible.is translation."),
    mapFn("study_light", "STUDY_LIGHT_TRANSLATION_MAP", "StudyLight translation."),
    mapFn("study_bible", "STUDY_BIBLE_TRANSLATION_MAP", "studybible.info translation."),
    mapFn("biblia", "BIBLIA_TRANSLATION_MAP", "Biblia translation."),
    mapFn("literal_word", "LITERAL_WORD_TRANSLATION_MAP", "LiteralWord translation."),
    `\n/// OpenBible geo chapters that have places, by geo book slug.\n#[rustfmt::skip]\npub fn open_bible_geo_chapters(slug: &str) -> &'static [u8] {\n    match slug {\n${geoArms.join("\n")}\n        _ => &[],\n    }\n}\n`,
  ].join("");
}
const services = [
  "bible-gateway", "slug", "bible-hub", "biblica", "enduring-word", "blue-letter-bible",
  "type-the-word", "catena", "step-bible", "logos-ref-ly", "sermon-audio", "bible-is",
] as const;

// Open Bible geo slugs aren't exported; read them from the (built) context through a probe passage.
const rows = OSIS_BOOK_CODES.map((code) => {
  const p = parsePassage(`${OSIS_BOOK_NAMES[code]} 1`);
  const geo = buildAdapterTemplateContext(p, "BSB").openBibleGeoPath ?? "";
  const geoSlug = geo.split(".")[0];
  const cells = services.map((s) => JSON.stringify(getServiceBookVariant(s, code)));
  return `    BookVariants { code: ${JSON.stringify(code)}, name: ${cells[0]}, slug: ${cells[1]}, bible_hub: ${cells[2]}, biblica: ${cells[3]}, enduring_word: ${cells[4]}, blue_letter_bible: ${cells[5]}, type_the_word: ${cells[6]}, catena: ${cells[7]}, step_bible: ${cells[8]}, logos_ref_ly: ${cells[9]}, sermon_audio: ${cells[10]}, bible_is: ${cells[11]}, bible_project_guide: ${JSON.stringify(getBibleProjectGuideSlug(code) ?? "")}, open_bible_geo: ${JSON.stringify(geoSlug)} },`;
});
writeFileSync(`${out}/crates/core/src/open_in_tables.rs`, `//! Lookup tables for Open in › link expansion, exactly as route.bible builds them
//! (\`book-variants.ts\`, \`context.ts\`). Generated by \`scripts/route-bible-parity\`; do not edit.

/// One book's spelling for every service variant route.bible knows.
pub struct BookVariants {
    pub code: &'static str,
    pub name: &'static str,
    pub slug: &'static str,
    pub bible_hub: &'static str,
    pub biblica: &'static str,
    pub enduring_word: &'static str,
    pub blue_letter_bible: &'static str,
    pub type_the_word: &'static str,
    pub catena: &'static str,
    pub step_bible: &'static str,
    pub logos_ref_ly: &'static str,
    pub sermon_audio: &'static str,
    pub bible_is: &'static str,
    /// BibleProject guide slug; empty when the book has no guide (search instead).
    pub bible_project_guide: &'static str,
    /// OpenBible geo slug; empty when the site has no page for the book.
    pub open_bible_geo: &'static str,
}

#[rustfmt::skip]
pub static BOOK_VARIANTS: [BookVariants; 66] = [
${rows.join("\n")}
];
${tables()}`);

const links = JSON.parse(readFileSync(`${out}/crates/core/data/adapters.links.json`, "utf8"));
const webTemplates: Record<string, string> = {};
for (const a of links.adapters) {
  const usable = a.links.filter((l: any) => !l.platforms?.length || l.platforms.includes("web"));
  const web = usable.find((l: any) => l.kind === "web") ?? usable[0];
  if (web) webTemplates[a.id] = web.template;
}
const casesFor = (inputs: [string, string][], ids?: string[], keys?: string[]) =>
  inputs.map(([input, translation]) => {
    const p = parsePassage(input);
    const all = buildAdapterTemplateReplacements(buildAdapterTemplateContext(p, translation));
    const vars: Record<string, string> = {};
    for (const k of keys ?? Object.keys(all)) vars[k] = all[k];
    const urls: Record<string, string> = {};
    for (const id of ids ?? Object.keys(webTemplates)) {
      if (webTemplates[id]) urls[id] = renderAdapterTemplate(webTemplates[id], buildAdapterTemplateContext(p, translation, id));
    }
    return { input, canonical: p.canonical, translation, vars, urls };
  });

// Committed fixture: a few passages that exercise the awkward cases, and only the adapters and
// template values whose rules are irregular (book spellings, ranges, translation maps).
const CASES: [string, string][] = [
  ["John 3:16", "BSB"], ["John 3:16", "KJV"], ["Jonah 2:3", "BSB"], ["Jonah 2:3", "ESV"],
  ["Ezekiel 18", "BSB"], ["Song of Songs 1", "WEB"], ["Psalm 23", "ESV"], ["Psalm 119:105", "NASB"],
  ["Philemon 1:6", "BSB"], ["1 John 2", "NIV"], ["Romans 5:3-5", "NKJV"], ["Genesis 1", "KJV"],
  ["Obadiah 1:3", "YLT"], ["Jude 1:24", "BSB"],
];
const ADAPTERS = [
  "wedevote", "blue-letter-bible", "bible-gateway", "bible-app", "catena", "step-bible",
  "open-bible-geo", "hidden-arrow", "verse-notes", "bible-project", "logos", "bible-is",
];
const VARS = [
  "passage", "displayPassage", "bookSlug", "versePath", "chapterPath", "verseRange",
  "youversionVersionId", "youversionTranslation", "catenaPath", "stepBibleQuery",
  "openBibleGeoPath", "bibleProjectPath", "logosRefLyReference", "bibleIsPath", "obsidianNoteName",
];
const cases = casesFor(CASES, ADAPTERS, VARS);

// Optional full sweep (every adapter and value, 66 books, many translations): written under
// target/ (not committed); `cargo test -p bible-core --test open_in_parity -- --ignored` reads it.
if (process.env.ROUTE_BIBLE_PARITY_FULL) {
  const inputs: [string, string][] = [];
  const tricky = [
    "Jonah 2:3", "Ezekiel 37:4", "Song of Songs 2:4", "Psalm 23", "Psalm 23:1-3", "Psalm 119:105",
    "Philemon 1:6", "1 John 4:8", "Judges 6:12", "Joel 2:28", "Nahum 1:7", "John 3:16",
    "Romans 5:3-5", "Genesis 1", "Revelation 22:21", "2 Thessalonians 3:16", "Obadiah 1:3", "Jude 1:3",
  ];
  for (const input of tricky) for (const t of ["BSB", "ESV", "KJV", "WEB"]) inputs.push([input, t]);
  for (const t of ["ASV", "YLT", "DBY", "WBS", "NKJV", "NIV", "NLT", "CSB", "HCSB", "NASB", "NASB95", "NASB1995", "NRSV", "NRSV-CI", "NRSVUE", "NET", "WEBUS", "LEB", "LSB", "DARBY"]) inputs.push(["John 3:16", t]);
  for (const code of OSIS_BOOK_CODES) inputs.push([`${OSIS_BOOK_NAMES[code]} 1:1`, "BSB"]);
  const full = casesFor(inputs);
  const path = `${out}/target/route-bible-parity-full.json`;
  mkdirSync(`${out}/target`, { recursive: true });
  writeFileSync(path, JSON.stringify({ source: `route.bible adapters ${links.version}`, cases: full }) + "\n");
  console.log(`route-bible-parity: full sweep (${full.length} cases) -> ${path}`);
}
// {{verseText}}: route.bible's own cleanup of local text (lines are verses).
const JOHN_3_16_KJV =
  "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.";
const longText = Array.from({ length: 120 }, (_, i) => `word${i}`).join(" ");
const verseTextInputs: string[][] = [
  [JOHN_3_16_KJV],
  ["16 " + JOHN_3_16_KJV],
  ["12 I know how to live humbly,", "[13] I can do all things through Christ who gives me strength."],
  ["v. 4 Rejoice in the Lord always. ¶ Again I say, rejoice!", "3:5 Let your gentleness be known*"],
  ["  spaced   out\ttext  ", "1000 men", "7) Seven"],
  ["   "],
  [longText],
];
const verseText = verseTextInputs.map((input) => ({ input, output: buildVerseQueryText(input) }));
const ha = links.adapters.find((a: any) => a.id === "hidden-arrow");
const hiddenArrow = ha
  ? (() => {
      const p = parsePassage("John 3:16");
      const template = webTemplates["hidden-arrow"];
      return {
        input: "John 3:16",
        text: JOHN_3_16_KJV,
        url: renderAdapterTemplate(template, buildAdapterTemplateContext(p, "KJV", "hidden-arrow", p, { verseText: [JOHN_3_16_KJV] })),
        fallback: renderAdapterTemplate(template, buildAdapterTemplateContext(p, "KJV", "hidden-arrow")),
      };
    })()
  : null;
// One case per line keeps the committed fixture small and reviewable.
const line = (v: unknown) => "  " + JSON.stringify(v);
writeFileSync(
  `${out}/crates/core/tests/fixtures/route-bible-parity.json`,
  [
    "{",
    `"source": ${JSON.stringify(`route.bible adapters ${links.version}`)},`,
    `"hiddenArrow": ${JSON.stringify(hiddenArrow)},`,
    '"verseText": [',
    verseText.map(line).join(",\n"),
    "],",
    '"cases": [',
    cases.map(line).join(",\n"),
    "]",
    "}",
  ].join("\n") + "\n",
);
console.log(`route-bible-parity: ${rows.length} books, ${cases.length} cases`);
