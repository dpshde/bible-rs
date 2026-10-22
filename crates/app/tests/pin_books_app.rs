//! Pinning books in the running app: right-clicking a book in a testament pins it above the
//! testaments, the pinned row opens the book where it was last read, and right-clicking it
//! unpins it. Its own test binary because it sets `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::theme;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Shape};
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);
const PHILIPPIANS: BookId = BookId(49);

/// Where the visible text `label` is drawn, top to bottom.
fn find(h: &Harness<'_, BibleApp>, label: &str) -> Vec<Pos2> {
    fn walk(shape: &Shape, clip: egui::Rect, label: &str, out: &mut Vec<Pos2>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, label, out)),
            Shape::Text(t) if t.galley.text() == label => {
                let r = t.visual_bounding_rect();
                if clip.contains(r.center()) {
                    out.push(r.center());
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for c in &h.output().shapes {
        walk(&c.shape, c.clip_rect, label, &mut out);
    }
    out.sort_by(|a, b| a.y.total_cmp(&b.y));
    out
}

fn one(h: &Harness<'_, BibleApp>, label: &str) -> Pos2 {
    let found = find(h, label);
    assert_eq!(found.len(), 1, "{label:?} drawn at {found:?}");
    found[0]
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2, button: PointerButton) {
    h.event(Event::PointerMoved(at));
    h.step();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(4);
}

#[test]
fn a_pinned_book_sits_above_the_testaments() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-pin-books-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.sidebar_open = true;
    let john3 = ChapterIdx::new(JOHN, 3).unwrap();
    init.settings.chapter = john3.0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 1000.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(8);

    // Reading John opens the New Testament; right-click Philippians and pin it.
    let row = one(&h, "Philippians");
    click(&mut h, row, PointerButton::Secondary);
    let pin = one(&h, "Pin to Top");
    click(&mut h, pin, PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [PHILIPPIANS]);

    // It now shows twice: pinned above the Old Testament, and still in the New Testament.
    let old = one(&h, "Old Testament");
    let rows = find(&h, "Philippians");
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(rows[0].y < old.y && rows[1].y > old.y, "{rows:?} vs Old Testament at {old:?}");

    // The pinned row opens it at chapter 1 (never read), then John pinned opens at John 3.
    click(&mut h, rows[0], PointerButton::Primary);
    assert_eq!(h.state().reader.location.0, PHILIPPIANS.first_chapter());
    let john = find(&h, "John");
    let john_row = *john.iter().find(|p| p.y > old.y).expect("John in the New Testament");
    click(&mut h, john_row, PointerButton::Secondary);
    let pin = one(&h, "Pin to Top");
    click(&mut h, pin, PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [PHILIPPIANS, JOHN], "in the order they were pinned");
    let john = find(&h, "John");
    let old = one(&h, "Old Testament");
    assert!(john[0].y < old.y && find(&h, "Philippians")[0].y < john[0].y);
    click(&mut h, john[0], PointerButton::Primary);
    assert_eq!(h.state().reader.location.0, john3, "where John was last read");

    // Right-clicking a pinned row unpins it.
    let pinned = find(&h, "Philippians")[0];
    click(&mut h, pinned, PointerButton::Secondary);
    let unpin = one(&h, "Unpin");
    click(&mut h, unpin, PointerButton::Primary);
    assert_eq!(h.state().settings.pinned(), [JOHN]);
    let _ = std::fs::remove_dir_all(&dir);
}
