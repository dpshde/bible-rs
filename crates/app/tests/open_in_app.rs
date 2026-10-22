//! Open in › in the running app: while its flyout is open, keys filter destinations instead of
//! reaching the reader (Backspace never clears a highlight, J/K never move the selection); words
//! chosen in a parallel column go out in that column's translation; and a menu with many stars
//! stays inside the window. Its own test binary because it sets `BIBLE_RS_OFFSCREEN` (see
//! `book_picker_app.rs`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_rs::app::{BibleApp, Startup, initialize, online_counter};
use bible_rs::reader::TextPos;
use bible_rs::theme;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;

const JOHN: BookId = BookId(42);

fn press(h: &mut Harness<'_, BibleApp>, pos: Pos2, button: PointerButton, pressed: bool) {
    h.event(Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE });
    h.step();
}

fn click(h: &mut Harness<'_, BibleApp>, at: Pos2, button: PointerButton) {
    h.event(Event::PointerMoved(at));
    h.step();
    press(h, at, button, true);
    press(h, at, button, false);
    h.run_steps(2);
}

fn drag(h: &mut Harness<'_, BibleApp>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.step();
    press(h, from, PointerButton::Primary, true);
    for i in 1..=8 {
        h.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        h.step();
    }
    press(h, to, PointerButton::Primary, false);
    h.run_steps(2);
}

fn key(h: &mut Harness<'_, BibleApp>, key: Key) {
    h.key_press(key);
    h.run_steps(2);
}

/// A letter typed: its key and its text, as a keyboard sends them.
fn letter(h: &mut Harness<'_, BibleApp>, k: Key, c: &str) {
    h.event(Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.event(Event::Text(c.into()));
    h.event(Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

#[test]
fn open_in_keeps_the_keyboard_and_the_columns_translation() {
    // SAFETY: the only test in this binary, so nothing else reads the environment meanwhile.
    unsafe { std::env::set_var("BIBLE_RS_OFFSCREEN", "1") };
    let dir = std::env::temp_dir().join(format!("bible-rs-open-in-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut init = initialize(dir.clone(), Arc::new(|| {})).expect("loading data");
    init.settings.onboarded = true;
    init.settings.translation = "kjv".into();
    init.settings.parallel = vec!["web".into()];
    init.settings.parallel_on = true;
    let c = ChapterIdx::new(JOHN, 3).unwrap();
    init.settings.chapter = c.0;
    let startup =
        Startup { t0: Instant::now(), probe: false, init: Duration::ZERO, first_ui: None, frames: 0 };
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_eframe(|cc| {
        cc.egui_ctx.set_fonts(theme::font_definitions());
        BibleApp::new(cc, init, online_counter(), startup)
    });
    h.run_steps(4);
    h.state_mut().reader.jump(c, Some(16), false);
    h.run_steps(6);
    let v = |n: u8| VerseRef::new(JOHN, 3, n);
    let at = |h: &Harness<'_, BibleApp>, n: u8, offset: usize, column: usize| {
        h.state().reader.point_at(TextPos { verse: v(n), offset, column }).expect("on screen")
    };

    // Verse 16 highlighted, then its menu and the Open in › flyout from the keyboard.
    let p = at(&h, 16, 4, 0);
    click(&mut h, p, PointerButton::Primary);
    key(&mut h, Key::Num1);
    assert!(!h.state().study.highlight_parts(v(16)).is_empty());
    click(&mut h, p, PointerButton::Secondary);
    let offset = h.state().reader.anchor.offset;
    // Note, Bookmark, Copy, Copy with Reference, Share, Compare, Parallel, Open in.
    for _ in 0..8 {
        key(&mut h, Key::ArrowDown);
    }
    key(&mut h, Key::ArrowRight);
    assert!(h.state().open_in_flyout_open(), "→ on Open in opens the flyout");
    assert_eq!(h.state().reader.anchor.offset, offset, "the menu's arrows don't scroll the reader");

    // Typing in the flyout filters it; none of it reaches the reader.
    key(&mut h, Key::ArrowDown);
    letter(&mut h, Key::J, "j");
    assert_eq!(h.state().reader.selection, Some((v(16), v(16))), "J doesn't move the selection");
    letter(&mut h, Key::K, "k");
    key(&mut h, Key::Backspace);
    key(&mut h, Key::Backspace);
    assert!(h.state().open_in_flyout_open());
    assert_eq!(h.state().reader.selection, Some((v(16), v(16))), "J/K don't move the selection");
    assert!(!h.state().study.highlight_parts(v(16)).is_empty(), "Backspace doesn't clear the highlight");
    assert_eq!(h.state().reader.anchor.offset, offset);
    // Escape closes the flyout, then the menu; the verse stays selected.
    key(&mut h, Key::Escape);
    assert!(!h.state().open_in_flyout_open());
    key(&mut h, Key::Escape);
    assert_eq!(h.state().reader.selection, Some((v(16), v(16))));

    // Words chosen in the WEB column go out as WEB; the whole verse in the reader's KJV.
    let url = |h: &Harness<'_, BibleApp>| h.state().open_in_url("google-translate").unwrap();
    assert!(url(&h).contains("begotten"), "{}", url(&h));
    let web = bible_core::bundled::load("web").unwrap().find(v(16)).unwrap().text.to_string();
    let words = web.find("only").unwrap();
    let (a, b) =
        (at(&h, 16, web[..words].chars().count(), 1), at(&h, 16, web[..words].chars().count() + 4, 1));
    drag(&mut h, a, b);
    assert_eq!(h.state().reader.text_span().map(|(_, id)| id.to_string()).as_deref(), Some("web"));
    assert!(!url(&h).contains("begotten"), "{}", url(&h));

    // Eighteen stars: the menu fits the window (and scrolls) instead of running off the bottom.
    let catalog = bible_core::open_in::Catalog::bundled();
    h.state_mut().settings.open_in_starred =
        catalog.menu_items().iter().take(18).map(|a| a.id.clone()).collect();
    let p = at(&h, 16, 4, 0);
    click(&mut h, p, PointerButton::Secondary);
    let menu = h.ctx.memory(|m| m.area_rect(egui::Id::new("verse-menu"))).expect("the menu is open");
    assert!(menu.top() >= 8.0 && menu.bottom() <= 792.0, "{menu:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
