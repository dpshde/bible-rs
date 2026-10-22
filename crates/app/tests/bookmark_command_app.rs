//! The `bookmark` command saves verses through the existing study store. Its own test binary
//! because it sets `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`). Palette parsing of
//! `bookmark <ref>` is covered by unit tests in `palette.rs`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui;
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);
const PHIL: BookId = BookId(49);

#[test]
fn bookmark_command_saves_a_verse_and_a_range() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe {
        std::env::set_var("BIBLE_RS_OFFSCREEN", "1");
        std::env::set_var("BIBLE_RS_BACKGROUND", "1");
        std::env::set_var("BIBLE_RS_DEMO", "bookmark:John 3:16;bookmark:Phil 4:11-13");
    }
    let dir = std::env::temp_dir().join(format!("bible-rs-bookmark-cmd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.chapter = ChapterIdx::new(JOHN, 3).unwrap().0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(16);
    assert!(h.state().study.is_bookmarked(VerseRef::new(JOHN, 3, 16)), "John 3:16 bookmarked");
    for n in [11u8, 12, 13] {
        assert!(h.state().study.is_bookmarked(VerseRef::new(PHIL, 4, n)), "Philippians 4:{n} bookmarked");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
