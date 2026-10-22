//! Jumping to a verse range selects every verse in it, and shift-dragging across verses selects
//! a range the same way, so a colour key highlights them all. Its own test binary because it sets
//! `BIBLE_RS_OFFSCREEN` (see `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_core::study::HighlightColor;
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::reader::TextPos;
use bible_rs::theme;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;

const PHIL: BookId = BookId(49);

fn press(h: &mut Harness<'_, BibleApp>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
    h.step();
}

fn shift_drag(h: &mut Harness<'_, BibleApp>, from: Pos2, to: Pos2) {
    h.event(Event::ModifiersChanged(Modifiers::SHIFT));
    h.event(Event::PointerMoved(from));
    h.step();
    press(h, from, true, Modifiers::SHIFT);
    for i in 1..=8 {
        h.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        h.step();
    }
    press(h, to, false, Modifiers::SHIFT);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, BibleApp>, key: Key) {
    h.key_press(key);
    h.run_steps(2);
}

#[test]
fn jumping_to_a_range_selects_every_verse() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe {
        std::env::set_var("BIBLE_RS_OFFSCREEN", "1");
        std::env::set_var("BIBLE_RS_BACKGROUND", "1");
        std::env::set_var("BIBLE_RS_DEMO", "jump:Phil 4:11-13");
    }
    let dir = std::env::temp_dir().join(format!("bible-rs-verse-range-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    let c = ChapterIdx::new(PHIL, 4).unwrap();
    init.settings.chapter = c.0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    // Demo steps start after a few frames; give the jump time to land.
    h.run_steps(12);

    let v = |n: u8| VerseRef::new(PHIL, 4, n);
    assert_eq!(h.state().reader.selected_refs(), vec![v(11), v(12), v(13)]);

    key(&mut h, Key::Num1);
    for n in [11, 12, 13] {
        assert_eq!(h.state().study.highlight(v(n)), Some(HighlightColor::Yellow), "verse {n} is highlighted");
    }

    // Shift-drag from 16 to 18 selects that range; a colour covers each whole verse.
    let john = ChapterIdx::new(BookId(42), 3).unwrap();
    h.state_mut().reader.jump(john, Some(16), false);
    h.run_steps(6);
    let jv = |n: u8| VerseRef::new(BookId(42), 3, n);
    let at = |h: &Harness<'_, BibleApp>, n: u8| {
        h.state().reader.point_at(TextPos { verse: jv(n), offset: 1, column: 0 }).expect("on screen")
    };
    let (p, q) = (at(&h, 16), at(&h, 18));
    shift_drag(&mut h, p, q);
    assert_eq!(h.state().reader.selected_refs(), vec![jv(16), jv(17), jv(18)]);
    assert!(h.state().reader.text_span().is_none(), "shift-drag selects verses, not words");
    key(&mut h, Key::Num2);
    for n in [16, 17, 18] {
        assert_eq!(h.state().study.highlight(jv(n)), Some(HighlightColor::Green), "verse {n}");
    }

    // Single click still selects one verse.
    let p = at(&h, 16);
    h.event(Event::PointerMoved(p));
    h.step();
    press(&mut h, p, true, Modifiers::NONE);
    press(&mut h, p, false, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().reader.selected_refs(), vec![jv(16)]);

    let _ = std::fs::remove_dir_all(&dir);
}
