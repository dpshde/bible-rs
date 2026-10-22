//! Bookmarking in the running app: B on verses selected across a stretch makes one bookmark of
//! them, on words dragged over one of just those words; the Bookmarks page shows each as one row
//! ("Philippians 4:11–13"), clicking it selects its verses, and B on any of its verses removes all
//! of it. Its own test binary because it sets `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::pages::Page;
use bible_rs::reader::TextPos;
use bible_rs::theme;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape};
use egui_kittest::Harness;

const PHILIPPIANS: BookId = BookId(49);

fn press(h: &mut Harness<'_, BibleApp>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
    h.step();
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.step();
    press(h, at, true, modifiers);
    press(h, at, false, modifiers);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn drag(h: &mut Harness<'_, BibleApp>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.step();
    press(h, from, true, Modifiers::NONE);
    for i in 1..=8 {
        h.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        h.step();
    }
    press(h, to, false, Modifiers::NONE);
    h.run_steps(2);
}

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

#[test]
fn bookmarks_cover_stretches_and_words() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-bookmark-ranges-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    let c = ChapterIdx::new(PHILIPPIANS, 4).unwrap();
    init.settings.chapter = c.0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(4);
    h.state_mut().reader.jump(c, Some(11), false);
    h.run_steps(6);
    let v = |n: u8| VerseRef::new(PHILIPPIANS, 4, n);
    let at = |h: &Harness<'_, BibleApp>, n: u8, offset: usize| {
        h.state().reader.point_at(TextPos { verse: v(n), offset, column: 0 }).expect("on screen")
    };

    // 11 through 13, selected and bookmarked with B: one bookmark.
    let p = at(&h, 11, 3);
    click(&mut h, p, Modifiers::NONE);
    let p = at(&h, 13, 3);
    click(&mut h, p, Modifiers::SHIFT);
    assert_eq!(h.state().reader.selection, Some((v(11), v(13))));
    h.key_press(Key::B);
    h.run_steps(2);
    let study = &h.state().study;
    assert_eq!(study.bookmarks().len(), 1);
    assert_eq!(study.bookmark_at(v(12)).map(|(k, b)| (k, b.last)), Some((v(11), 13)));

    // Words of 19, dragged over and bookmarked: just them.
    let kjv = bible_core::bundled::load("kjv").unwrap();
    let t19 = kjv.find(v(19)).unwrap().text.to_string();
    let words = "my God shall supply all your need";
    let a = t19[..t19.find(words).unwrap()].chars().count();
    h.state_mut().reader.jump(c, Some(19), false);
    h.run_steps(6);
    let (p, q) = (at(&h, 19, a), at(&h, 19, a + words.chars().count()));
    drag(&mut h, p, q);
    h.key_press(Key::B);
    h.run_steps(2);
    let b = h.state().study.bookmark(v(19)).cloned().expect("bookmarked");
    assert_eq!(b.quote.as_deref(), Some(words));

    // The page: a row each, the stretch under its range.
    h.state_mut().pages.open(Page::Bookmarks);
    h.run_steps(4);
    assert!(painted(&h, "2 bookmarks in 1 book").is_some());
    let row = painted(&h, "Philippians 4:11–13").expect("one row for 11–13");
    assert!(painted(&h, "Philippians 4:12").is_none());
    assert!(painted(&h, "Philippians 4:19").is_some());
    click(&mut h, row.center(), Modifiers::NONE);
    assert!(h.state().pages.reader());
    assert_eq!(h.state().reader.selection, Some((v(11), v(13))));

    // B on one of its verses removes all of it.
    let p = at(&h, 12, 3);
    click(&mut h, p, Modifiers::NONE);
    assert_eq!(h.state().reader.selection, Some((v(12), v(12))));
    h.key_press(Key::B);
    h.run_steps(2);
    let study = &h.state().study;
    assert!((11..=13).all(|n| !study.is_bookmarked(v(n))));
    assert_eq!(study.bookmarks().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
