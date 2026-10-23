//! The verse menu (frame 08a): right-click a verse for the highlight swatches (1–6, 0 clears),
//! note, bookmark, copy, share, compare, parallel and Open in ›.

use bible_core::VerseRef;
use bible_core::open_in::{APPS, Passage};
use bible_core::study::{HighlightColor, StudyStore};
use eframe::egui::{
    self, Align2, Color32, Key, Modifiers, Order, Painter, Pos2, Rect, Sense, Stroke, pos2, vec2,
};

use crate::actions::Action;
use crate::icons;
use crate::menu::{self, ITEM_HEIGHT, PADDING_X, PADDING_Y, SEPARATOR_HEIGHT};
use crate::theme::{Palette, menu as tokens, text};
use crate::widgets::IconFn;

const WIDTH: f32 = 264.0;
/// The swatch row under the header: 20 pt swatches, 10 pt apart, 6 pt above and 8 below.
const SWATCHES_HEIGHT: f32 = 34.0;
/// The Open in › submenu: one text row per app.
const OPEN_IN_WIDTH: f32 = 200.0;

enum Line {
    Item {
        icon: IconFn,
        label: String,
        shortcut: &'static str,
        action: Option<Action>,
    },
    /// Shows the Open in › submenu; off for a selection across chapters.
    OpenIn {
        enabled: bool,
    },
    Separator,
}

pub struct VerseMenu {
    pos: Pos2,
    /// The item under the keyboard (and the pointer).
    highlighted: Option<usize>,
    opened: bool,
    /// The Open in › submenu, while open: the app under the keyboard (and the pointer).
    open_in: Option<Option<usize>>,
}

impl VerseMenu {
    pub fn new(pos: Pos2) -> Self {
        Self { pos, highlighted: None, opened: false, open_in: None }
    }

    /// Starts with the first item highlighted, as when opened from the keyboard.
    pub fn with_first_highlighted(mut self) -> Self {
        self.highlighted = Some(0);
        self
    }

    fn lines(study: &StudyStore, refs: &[VerseRef]) -> Vec<Line> {
        let first = refs[0];
        let item = |icon: IconFn, label: &str, shortcut: &'static str, action: Option<Action>| Line::Item {
            icon,
            label: label.to_string(),
            shortcut,
            action,
        };
        vec![
            item(
                icons::note,
                if study.note(first).is_some() { "Edit Note…" } else { "Add Note…" },
                "N",
                Some(Action::EditSelectedNote),
            ),
            item(
                bookmark,
                if study.is_bookmarked(first) { "Remove Bookmark" } else { "Bookmark" },
                "B",
                Some(Action::ToggleBookmark),
            ),
            Line::Separator,
            item(icons::copy, "Copy", "⌘C", Some(Action::CopyText)),
            item(icons::quote, "Copy with Reference", "⇧⌘C", Some(Action::Copy)),
            item(icons::share, "Share as Image…", "", Some(Action::ShareImage)),
            Line::Separator,
            item(icons::compare, "Compare Translations", "C", Some(Action::Compare)),
            item(icons::columns, "Open in Parallel", "⇧⌘P", Some(Action::ToggleParallel)),
            Line::OpenIn { enabled: Passage::from_refs(refs).is_some() },
            Line::Separator,
            item(icons::text_cursor, "Select Verse Range", "⇧-click", None),
        ]
    }

    /// Returns `true` when the menu should close.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        study: &StudyStore,
        refs: &[VerseRef],
        // The colour all of the selection has, if one: its swatch is checked, and clears.
        current: Option<HighlightColor>,
        settings: &crate::app::Settings,
        actions: &mut Vec<Action>,
    ) -> bool {
        if refs.is_empty() {
            return true;
        }
        let lines = Self::lines(study, refs);
        let enabled: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| matches!(l, Line::Item { action: Some(_), .. } | Line::OpenIn { enabled: true }))
            .map(|(i, _)| i)
            .collect();
        let (down, up, enter, esc, left, right) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.consume_key(Modifiers::NONE, Key::ArrowLeft),
                i.consume_key(Modifiers::NONE, Key::ArrowRight),
            )
        });
        let open_in_line = lines.iter().position(|l| matches!(l, Line::OpenIn { enabled: true }));
        if let Some(sub) = &mut self.open_in {
            // The submenu has the keyboard: ← or Esc goes back to the menu.
            let n = APPS.len();
            if down {
                *sub = Some(sub.map_or(0, |a| (a + 1) % n));
            }
            if up {
                *sub = Some(sub.map_or(n - 1, |a| (a + n - 1) % n));
            }
            if enter && let Some(a) = *sub {
                actions.push(Action::OpenIn(APPS[a].id));
                return true;
            }
            if left || esc {
                self.open_in = None;
            }
        } else {
            if esc {
                return true;
            }
            let at = self.highlighted.and_then(|h| enabled.iter().position(|i| *i == h));
            if down {
                self.highlighted = Some(enabled[at.map_or(0, |a| (a + 1) % enabled.len())]);
            }
            if up {
                self.highlighted =
                    Some(enabled[at.map_or(enabled.len() - 1, |a| (a + enabled.len() - 1) % enabled.len())]);
            }
            if (enter || right) && self.highlighted.is_some() && self.highlighted == open_in_line {
                self.open_in = Some(Some(0));
            } else if enter
                && let Some(Line::Item { action: Some(a), .. }) = self.highlighted.and_then(|h| lines.get(h))
            {
                actions.push(a.clone());
                return true;
            }
        }

        let height = PADDING_Y
            + tokens::HEADER_HEIGHT
            + SWATCHES_HEIGHT
            + SEPARATOR_HEIGHT
            + lines
                .iter()
                .map(|l| if matches!(l, Line::Separator) { SEPARATOR_HEIGHT } else { ITEM_HEIGHT })
                .sum::<f32>()
            + PADDING_Y;
        let screen = ctx.content_rect();
        let pos = pos2(
            self.pos.x.min(screen.right() - WIDTH - 8.0).max(screen.left() + 8.0),
            self.pos.y.min(screen.bottom() - height - 8.0).max(screen.top() + 8.0),
        );
        let mut close = false;
        let mut open_in_row: Option<Rect> = None;
        let area = egui::Area::new(egui::Id::new("verse-menu")).order(Order::Foreground).fixed_pos(pos).show(
            ctx,
            |ui| {
                let rect = Rect::from_min_size(pos, vec2(WIDTH, height));
                p.shadows.popover.paint(ui.painter(), rect, tokens::RADIUS);
                ui.painter().rect_filled(rect, tokens::RADIUS, p.surface);
                let (_, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                let (left, right) = (rect.left() + PADDING_X, rect.right() - PADDING_X);
                let mut y = rect.top() + PADDING_Y;
                // "Highlight", then the swatches and clear.
                let header = Rect::from_x_y_ranges(left..=right, y..=y + tokens::HEADER_HEIGHT);
                menu::paint_header(ui.painter(), p, header, "Highlight");
                y += tokens::HEADER_HEIGHT;
                let cy = y + 6.0 + 10.0;
                let mut x = menu::text_left(header);
                for (k, c) in HighlightColor::ALL.into_iter().enumerate() {
                    let center = pos2(x + 10.0, cy);
                    let resp = ui.interact(
                        Rect::from_center_size(center, vec2(24.0, 24.0)),
                        ui.id().with(("swatch", k)),
                        Sense::click(),
                    );
                    if current == Some(c) {
                        ui.painter().circle_stroke(center, 10.0 + 2.75, Stroke::new(1.5, p.ink));
                    } else if resp.hovered() {
                        ui.painter().circle_filled(center, 10.0 + 3.0, p.fill_strong);
                    }
                    ui.painter().circle_filled(center, 10.0, p.swatch(c));
                    ui.painter().circle_stroke(center, 9.75, Stroke::new(0.5, Color32::from_black_alpha(46)));
                    if current == Some(c) {
                        let s = Stroke::new(1.75, Color32::WHITE);
                        ui.painter().line_segment([center + vec2(-4.0, 0.25), center + vec2(-1.25, 3.0)], s);
                        ui.painter().line_segment([center + vec2(-1.25, 3.0), center + vec2(4.0, -2.75)], s);
                    }
                    if resp.on_hover_text(format!("{} ({})", settings.highlight_name(c), k + 1)).clicked() {
                        actions.push(Action::Highlight(if current == Some(c) { None } else { Some(c) }));
                        close = true;
                    }
                    x += 20.0 + 10.0;
                }
                let clear = Rect::from_min_size(pos2(x, cy - 10.0), vec2(20.0, 20.0));
                let resp = ui.interact(clear.expand(2.0), ui.id().with("swatch-clear"), Sense::click());
                if resp.hovered() {
                    ui.painter().circle_filled(clear.center(), 13.0, p.fill_strong);
                }
                icons::slash_circle(ui.painter(), clear, p.ink_3);
                if resp.on_hover_text("Remove highlight (0)").clicked() {
                    actions.push(Action::Highlight(None));
                    close = true;
                }
                y += SWATCHES_HEIGHT;
                menu::paint_separator(ui.painter(), p, left, right, y);
                y += SEPARATOR_HEIGHT;

                for (i, line) in lines.iter().enumerate() {
                    match line {
                        Line::Separator => {
                            menu::paint_separator(ui.painter(), p, left, right, y);
                            y += SEPARATOR_HEIGHT;
                        }
                        Line::Item { icon, label, shortcut, action } => {
                            let r = Rect::from_x_y_ranges(left..=right, y..=y + ITEM_HEIGHT);
                            let resp = ui.interact(r, ui.id().with(("menu-item", i)), Sense::click());
                            if action.is_some()
                                && resp.hovered()
                                && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO)
                            {
                                self.highlighted = Some(i);
                            }
                            let row = menu::Row {
                                icon: *icon,
                                label,
                                shortcut,
                                enabled: action.is_some(),
                                lit: self.highlighted == Some(i),
                                pressed: resp.is_pointer_button_down_on(),
                            };
                            menu::paint_row(ui.painter(), p, r, row);
                            if let Some(a) = action
                                && resp.clicked()
                            {
                                actions.push(a.clone());
                                close = true;
                            }
                            y += ITEM_HEIGHT;
                        }
                        Line::OpenIn { enabled } => {
                            let r = Rect::from_x_y_ranges(left..=right, y..=y + ITEM_HEIGHT);
                            let sense = if *enabled { Sense::click() } else { Sense::hover() };
                            let resp = ui.interact(r, ui.id().with(("menu-item", i)), sense);
                            // Pointing at the row opens the submenu, as in a macOS menu.
                            if *enabled
                                && (resp.clicked()
                                    || resp.hovered()
                                        && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO))
                            {
                                self.highlighted = Some(i);
                                self.open_in.get_or_insert(None);
                            }
                            let row = menu::Row {
                                icon: icons::external,
                                label: "Open in",
                                shortcut: "›",
                                enabled: *enabled,
                                lit: self.highlighted == Some(i) || self.open_in.is_some(),
                                pressed: resp.is_pointer_button_down_on(),
                            };
                            menu::paint_row(ui.painter(), p, r, row);
                            open_in_row = Some(r);
                            y += ITEM_HEIGHT;
                        }
                    }
                }
            },
        );
        // Pointing at another row of the menu closes the submenu.
        if self.open_in.is_some() && self.highlighted.is_some() && self.highlighted != open_in_line {
            self.open_in = None;
        }
        let mut in_submenu = false;
        if let (Some(sub), Some(row)) = (&mut self.open_in, open_in_row) {
            let (picked, inside) = open_in_submenu(ctx, p, row, pos.x, sub);
            in_submenu = inside;
            if let Some(a) = picked {
                actions.push(Action::OpenIn(APPS[a].id));
                close = true;
            }
        }
        if !self.opened {
            self.opened = true;
        } else if area.response.clicked_elsewhere() && !in_submenu {
            close = true;
        }
        close
    }
}

/// Paints the Open in › submenu beside `row` (or left of the menu at `menu_left` when the window
/// has no room on the right). Returns the app clicked, if any, and whether a click landed inside.
fn open_in_submenu(
    ctx: &egui::Context,
    p: &Palette,
    row: Rect,
    menu_left: f32,
    highlighted: &mut Option<usize>,
) -> (Option<usize>, bool) {
    let height = PADDING_Y + APPS.len() as f32 * ITEM_HEIGHT + PADDING_Y;
    let screen = ctx.content_rect();
    let beside = row.right() + PADDING_X;
    let x = if beside + OPEN_IN_WIDTH + 8.0 <= screen.right() {
        beside
    } else {
        (menu_left - OPEN_IN_WIDTH).max(screen.left() + 8.0)
    };
    let y = (row.top() - PADDING_Y).min(screen.bottom() - height - 8.0).max(screen.top() + 8.0);
    let pos = pos2(x, y);
    let mut picked = None;
    let area = egui::Area::new(egui::Id::new("verse-menu-open-in"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            let rect = Rect::from_min_size(pos, vec2(OPEN_IN_WIDTH, height));
            p.shadows.popover.paint(ui.painter(), rect, tokens::RADIUS);
            ui.painter().rect_filled(rect, tokens::RADIUS, p.surface);
            let _ = ui.allocate_exact_size(rect.size(), Sense::hover());
            let (left, right) = (rect.left() + PADDING_X, rect.right() - PADDING_X);
            for (k, app) in APPS.iter().enumerate() {
                let top = rect.top() + PADDING_Y + k as f32 * ITEM_HEIGHT;
                let r = Rect::from_x_y_ranges(left..=right, top..=top + ITEM_HEIGHT);
                let resp = ui.interact(r, ui.id().with(("open-in", k)), Sense::click());
                if resp.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) {
                    *highlighted = Some(k);
                }
                let lit = *highlighted == Some(k);
                if lit {
                    ui.painter().rect_filled(r, tokens::ITEM_RADIUS, p.accent);
                    if resp.is_pointer_button_down_on() {
                        ui.painter().rect_filled(r, tokens::ITEM_RADIUS, Color32::from_black_alpha(18));
                    }
                }
                let ink = if lit { p.on_accent } else { p.ink };
                let at = pos2(menu::text_left(r), r.center().y);
                ui.painter().text(at, Align2::LEFT_CENTER, app.name, text::UI.font(), ink);
                if resp.clicked() {
                    picked = Some(k);
                }
            }
        });
    let inside = ctx.input(|i| {
        i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|at| area.response.rect.contains(at))
    });
    (picked, inside)
}

fn bookmark(p: &Painter, r: Rect, c: Color32) {
    icons::bookmark(p, r, c, false);
}
