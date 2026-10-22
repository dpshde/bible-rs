//! The Highlights page shows a highlight across neighbouring verses as one card, labelled with the
//! range: clicking it selects the verses, and removing it removes every verse's highlight. Its own
//! test binary because it sets `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_core::study::{HighlightColor, TextSpan};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::pages::Page;
use bible_rs::theme;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

const PHILIPPIANS: BookId = BookId(49);

/// Where text reading `label` is painted, if anywhere.
fn painted(h: &Harness<'_, BibleApp>, label: &str) -> Option<Rect> {
    fn find(shape: &Shape, label: &str) -> Option<Rect> {
        match shape {
            Shape::Vec(v) => v.iter().find_map(|s| find(s, label)),
            Shape::Text(t) if t.galley.text() == label => Some(t.visual_bounding_rect()),
            _ => None,
        }
    }
    h.output().shapes.iter().find_map(|s| find(&s.shape, label))
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2, button: PointerButton) {
    h.event(Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(2);
}

#[test]
fn a_highlight_across_verses_is_one_card() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-highlight-runs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    let v = |n: u8| VerseRef::new(PHILIPPIANS, 4, n);
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let text = |r: VerseRef| kjv.find(r).map(|v| v.text.to_string());
    init.study.highlight_span(&TextSpan::verses(v(11), v(13)), Some(HighlightColor::Pink), text).unwrap();
    // Another colour next to it, and the same colour a verse past it, stay their own cards.
    init.study.set_highlight(v(14), Some(HighlightColor::Yellow)).unwrap();
    init.study.set_highlight(v(16), Some(HighlightColor::Pink)).unwrap();
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(4);
    h.state_mut().pages.open(Page::Highlights);
    h.run_steps(4);

    assert!(painted(&h, "3 highlights in 1 book").is_some(), "the page counts cards");
    let card = painted(&h, "Philippians 4:11–13").expect("one card for 11–13");
    assert!(painted(&h, "Philippians 4:11").is_none() && painted(&h, "Philippians 4:12").is_none());
    assert!(painted(&h, "Philippians 4:14").is_some() && painted(&h, "Philippians 4:16").is_some());

    // Clicking it goes to the verses and selects them.
    click(&mut h, card.center(), PointerButton::Primary);
    assert!(h.state().pages.reader());
    assert_eq!(h.state().reader.selection, Some((v(11), v(13))));
    assert_eq!(h.state().reader.anchor.chapter, ChapterIdx::new(PHILIPPIANS, 4).unwrap());

    // Removing it removes all three verses' highlights, and nothing else.
    h.state_mut().pages.open(Page::Highlights);
    h.run_steps(4);
    let card = painted(&h, "Philippians 4:11–13").unwrap();
    click(&mut h, card.center(), PointerButton::Secondary);
    h.get_by_label("Remove Highlight").click();
    h.run_steps(4);
    let study = &h.state().study;
    assert_eq!((11..=13).filter_map(|n| study.highlight(v(n))).count(), 0);
    assert_eq!(study.highlight(v(14)), Some(HighlightColor::Yellow));
    assert_eq!(study.highlight_runs().len(), 2);
    assert!(painted(&h, "2 highlights in 1 book").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}
