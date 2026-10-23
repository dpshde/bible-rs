//! The verse menu (frame 08a): right-click a verse for the highlight swatches (1–6, 0 clears),
//! note, bookmark, copy, share, compare, parallel and Open in ›.

use bible_core::VerseRef;
use bible_core::open_in::{self, App, Passage};
use bible_core::study::{HighlightColor, StudyStore};
use eframe::egui::{self, Color32, Key, Modifiers, Order, Painter, Pos2, Rect, Sense, Stroke, pos2, vec2};

use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};

use crate::actions::Action;
use crate::icons;
use crate::menu::{self, ITEM_HEIGHT, PADDING_X, PADDING_Y, SEPARATOR_HEIGHT};
use crate::theme::{Palette, icon, menu as tokens, text};
use crate::widgets::IconFn;

const WIDTH: f32 = 264.0;
/// The swatch row under the header: 20 pt swatches, 10 pt apart, 6 pt above and 8 below.
const SWATCHES_HEIGHT: f32 = 34.0;
/// The Open in › submenu's width bounds; between them it fits its longest label.
const OPEN_IN_MIN_WIDTH: f32 = 180.0;
const OPEN_IN_MAX_WIDTH: f32 = 280.0;
/// Kept free at the right of every app row, starred or not: the star and a gap before it.
const STAR_SLOT: f32 = icon::MENU + tokens::ITEM_PADDING;

/// The submenu width that fits a row's globe, a `label` wide label and the star slot.
fn open_in_width(label: f32) -> f32 {
    let row = tokens::ITEM_PADDING + icon::MENU + tokens::ICON_GAP + label + STAR_SLOT + tokens::ITEM_PADDING;
    (2.0 * PADDING_X + row).clamp(OPEN_IN_MIN_WIDTH, OPEN_IN_MAX_WIDTH)
}

/// The globe's colour for `app`: mostly the muted icon ink, with a hint of the app's brand colour.
/// A brand too dark to see on a dark menu is lifted toward white first.
fn globe_tint(p: &Palette, app: &App) -> Color32 {
    let [_, r, g, b] = app.brand.to_be_bytes();
    let lum = |c: Color32| (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0;
    let mut brand = Color32::from_rgb(r, g, b);
    if lum(p.surface) < 0.5 && lum(brand) < 0.35 {
        brand = mix(brand, Color32::WHITE, 0.45);
    }
    mix(p.ink_2, brand, GLOBE_BRAND_SHARE)
}

/// How much of the brand colour goes into a globe; the rest is the muted icon ink.
const GLOBE_BRAND_SHARE: f32 = 0.4;

/// `a` moved `t` of the way to `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

/// An app row: the globe, then the name, cut short with "…" before the star slot. On the lit row
/// the globe takes the selected foreground like the label.
fn paint_app_row(painter: &Painter, p: &Palette, r: Rect, app: &App, lit: bool, pressed: bool) {
    let name = app.name;
    if lit {
        painter.rect_filled(r, tokens::ITEM_RADIUS, p.accent);
        if pressed {
            painter.rect_filled(r, tokens::ITEM_RADIUS, Color32::from_black_alpha(18));
        }
    }
    let (ink, ink2) = if lit { (p.on_accent, p.on_accent) } else { (p.ink, globe_tint(p, app)) };
    let cy = r.center().y;
    let glyph =
        Rect::from_min_size(pos2(menu::text_left(r), cy - icon::MENU / 2.0), vec2(icon::MENU, icon::MENU));
    icons::heavy(|| icons::globe(painter, glyph, ink2));
    let left = glyph.right() + tokens::ICON_GAP;
    let mut job = LayoutJob::single_section(
        name.to_string(),
        TextFormat { font_id: text::UI.font(), color: ink, ..Default::default() },
    );
    job.wrap = TextWrapping {
        max_width: r.right() - tokens::ITEM_PADDING - STAR_SLOT - left,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);
    painter.galley(pos2(left, cy - galley.size().y / 2.0), galley, ink);
}

enum Line {
    Item {
        icon: IconFn,
        label: String,
        shortcut: &'static str,
        action: Option<Action>,
    },
    /// Shows the Open in › submenu; off for a selection across chapters.
    OpenIn(bool),
    /// A starred Open in › app, right under Open in ›: opens straight away.
    Starred(App),
    Separator,
}

pub struct VerseMenu {
    pos: Pos2,
    /// The item under the keyboard (and the pointer).
    highlighted: Option<usize>,
    opened: bool,
    /// The Open in › submenu, while open: the app under the keyboard (and the pointer).
    open_in: Option<Option<usize>>,
    /// How far the rows are scrolled when the menu is taller than the window (many stars).
    scroll_y: f32,
}

/// Above the rows: padding, the "Highlight" header, the swatches and their separator.
const ROWS_TOP: f32 = PADDING_Y + tokens::HEADER_HEIGHT + SWATCHES_HEIGHT + SEPARATOR_HEIGHT;

fn line_height(l: &Line) -> f32 {
    if matches!(l, Line::Separator) { SEPARATOR_HEIGHT } else { ITEM_HEIGHT }
}

/// The menu's top-left and visible height: it keeps 8 pt from the window's edges and, when its
/// `content` is taller than that, shows what fits and scrolls.
fn frame(at: Pos2, content: f32, screen: Rect) -> (Pos2, f32) {
    let height = content.min(screen.height() - 16.0).max(0.0);
    let pos = pos2(
        at.x.min(screen.right() - WIDTH - 8.0).max(screen.left() + 8.0),
        at.y.min(screen.bottom() - height - 8.0).max(screen.top() + 8.0),
    );
    (pos, height)
}

/// `scroll` moved just enough to show the row from `top` to `bottom` (content coordinates) in a
/// `view` tall menu, kept within `0..=content - view`.
fn scroll_to_show(scroll: f32, top: f32, bottom: f32, view: f32, content: f32) -> f32 {
    let s = if top < scroll + ROWS_TOP {
        top - ROWS_TOP
    } else if bottom > scroll + view - PADDING_Y {
        bottom - view + PADDING_Y
    } else {
        scroll
    };
    s.clamp(0.0, (content - view).max(0.0))
}

impl VerseMenu {
    pub fn new(pos: Pos2) -> Self {
        Self { pos, highlighted: None, opened: false, open_in: None, scroll_y: 0.0 }
    }

    /// Starts with the first item highlighted, as when opened from the keyboard.
    pub fn with_first_highlighted(mut self) -> Self {
        self.highlighted = Some(0);
        self
    }

    fn lines(study: &StudyStore, refs: &[VerseRef], starred: &[String]) -> Vec<Line> {
        let first = refs[0];
        let item = |icon: IconFn, label: &str, shortcut: &'static str, action: Option<Action>| Line::Item {
            icon,
            label: label.to_string(),
            shortcut,
            action,
        };
        let open_in = Passage::from_refs(refs).is_some();
        let mut lines = vec![
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
            Line::OpenIn(open_in),
        ];
        // Starred apps sit right under Open in ›, in star order.
        if open_in {
            lines.extend(open_in::starred(starred).into_iter().map(Line::Starred));
        }
        lines.extend([Line::Separator, item(icons::text_cursor, "Select Verse Range", "⇧-click", None)]);
        lines
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
        let lines = Self::lines(study, refs, &settings.open_in_starred);
        let order = open_in::menu_order(&settings.open_in_starred);
        let enabled: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                matches!(l, Line::Item { action: Some(_), .. } | Line::OpenIn(true) | Line::Starred(_))
            })
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
        let open_in_line = lines.iter().position(|l| matches!(l, Line::OpenIn(true)));
        if let Some(sub) = &mut self.open_in {
            // The submenu has the keyboard: ← or Esc goes back to the menu.
            let n = order.len();
            if down {
                *sub = Some(sub.map_or(0, |a| (a + 1) % n));
            }
            if up {
                *sub = Some(sub.map_or(n - 1, |a| (a + n - 1) % n));
            }
            if enter && let Some(a) = *sub {
                actions.push(Action::OpenIn(order[a].id));
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
            } else if enter {
                match self.highlighted.and_then(|h| lines.get(h)) {
                    Some(Line::Item { action: Some(a), .. }) => {
                        actions.push(a.clone());
                        return true;
                    }
                    Some(Line::Starred(app)) => {
                        actions.push(Action::OpenIn(app.id));
                        return true;
                    }
                    _ => {}
                }
            }
        }

        let content = ROWS_TOP + lines.iter().map(line_height).sum::<f32>() + PADDING_Y;
        let (pos, height) = frame(self.pos, content, ctx.content_rect());
        if self.open_in.is_none()
            && (down || up)
            && let Some(h) = self.highlighted
        {
            let top = ROWS_TOP + lines[..h].iter().map(line_height).sum::<f32>();
            self.scroll_y = scroll_to_show(self.scroll_y, top, top + ITEM_HEIGHT, height, content);
        }
        self.scroll_y = self.scroll_y.clamp(0.0, content - height);
        let mut close = false;
        let mut open_in_row: Option<Rect> = None;
        let area = egui::Area::new(egui::Id::new("verse-menu")).order(Order::Foreground).fixed_pos(pos).show(
            ctx,
            |ui| {
                let rect = Rect::from_min_size(pos, vec2(WIDTH, height));
                p.shadows.popover.paint(ui.painter(), rect, tokens::RADIUS);
                ui.painter().rect_filled(rect, tokens::RADIUS, p.surface);
                let (_, resp) = ui.allocate_exact_size(rect.size(), Sense::hover());
                if content > height && resp.contains_pointer() {
                    let dy = ui.ctx().input_mut(|i| std::mem::take(&mut i.smooth_scroll_delta.y));
                    self.scroll_y = (self.scroll_y - dy).clamp(0.0, content - height);
                }
                // Rows scrolled out of the menu neither paint nor take the pointer.
                ui.set_clip_rect(rect);
                let (left, right) = (rect.left() + PADDING_X, rect.right() - PADDING_X);
                let mut y = rect.top() + PADDING_Y - self.scroll_y;
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
                        Line::Starred(app) => {
                            let r = Rect::from_x_y_ranges(left..=right, y..=y + ITEM_HEIGHT);
                            let resp = ui.interact(r, ui.id().with(("menu-starred", app.id)), Sense::click());
                            if resp.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) {
                                self.highlighted = Some(i);
                            }
                            let lit = self.highlighted == Some(i);
                            let pressed = resp.is_pointer_button_down_on();
                            paint_app_row(ui.painter(), p, r, app, lit, pressed);
                            // Its filled star unstars it (the row goes) without opening.
                            let star = star_toggle(ui, p, r, app.id, true, resp.hovered() || lit, lit);
                            if star.clicked() {
                                actions.push(Action::ToggleOpenInStar(app.id));
                            } else if resp.clicked() {
                                actions.push(Action::OpenIn(app.id));
                                close = true;
                            }
                            y += ITEM_HEIGHT;
                        }
                        Line::OpenIn(enabled) => {
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
            let starred = &settings.open_in_starred;
            let (picked, inside) = open_in_submenu(ctx, p, row, pos.x, &order, starred, sub, actions);
            in_submenu = inside;
            if let Some(app) = picked {
                actions.push(Action::OpenIn(app.id));
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
/// has no room on the right), `order` top to bottom. Returns the app clicked, if any, and whether a
/// click landed inside. A row's star (shown on hover, kept when starred) stars or unstars it.
#[allow(clippy::too_many_arguments)]
fn open_in_submenu(
    ctx: &egui::Context,
    p: &Palette,
    row: Rect,
    menu_left: f32,
    order: &[App],
    starred: &[String],
    highlighted: &mut Option<usize>,
    actions: &mut Vec<Action>,
) -> (Option<App>, bool) {
    let height = PADDING_Y + order.len() as f32 * ITEM_HEIGHT + PADDING_Y;
    let longest = ctx.fonts_mut(|f| {
        order
            .iter()
            .map(|a| f.layout_no_wrap(a.name.to_string(), text::UI.font(), p.ink).size().x)
            .fold(0.0, f32::max)
    });
    let width = open_in_width(longest);
    let screen = ctx.content_rect();
    let beside = row.right() + PADDING_X;
    let x = if beside + width + 8.0 <= screen.right() {
        beside
    } else {
        (menu_left - width).max(screen.left() + 8.0)
    };
    let y = (row.top() - PADDING_Y).min(screen.bottom() - height - 8.0).max(screen.top() + 8.0);
    let pos = pos2(x, y);
    let mut picked = None;
    let area = egui::Area::new(egui::Id::new("verse-menu-open-in"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            let rect = Rect::from_min_size(pos, vec2(width, height));
            p.shadows.popover.paint(ui.painter(), rect, tokens::RADIUS);
            ui.painter().rect_filled(rect, tokens::RADIUS, p.surface);
            let _ = ui.allocate_exact_size(rect.size(), Sense::hover());
            let (left, right) = (rect.left() + PADDING_X, rect.right() - PADDING_X);
            for (k, app) in order.iter().enumerate() {
                let top = rect.top() + PADDING_Y + k as f32 * ITEM_HEIGHT;
                let r = Rect::from_x_y_ranges(left..=right, top..=top + ITEM_HEIGHT);
                let resp = ui.interact(r, ui.id().with(("open-in", app.id)), Sense::click());
                if resp.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) {
                    *highlighted = Some(k);
                }
                let lit = *highlighted == Some(k);
                paint_app_row(ui.painter(), p, r, app, lit, resp.is_pointer_button_down_on());
                let is_starred = starred.iter().any(|s| s == app.id);
                let star =
                    star_toggle(ui, p, r, app.id, is_starred, is_starred || resp.hovered() || lit, lit);
                if star.clicked() {
                    actions.push(Action::ToggleOpenInStar(app.id));
                } else if resp.clicked() {
                    picked = Some(*app);
                }
            }
        });
    let inside = ctx.input(|i| {
        i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|at| area.response.rect.contains(at))
    });
    (picked, inside)
}

/// The star at a row's trailing edge: filled when `starred`, drawn only when `visible` or itself
/// pointed at. Returns its response; the pointing hand shows over it, as it's clickable.
fn star_toggle(
    ui: &egui::Ui,
    p: &Palette,
    row: Rect,
    id: &str,
    starred: bool,
    visible: bool,
    lit: bool,
) -> egui::Response {
    let star = Rect::from_center_size(
        pos2(row.right() - tokens::ITEM_PADDING - icon::MENU / 2.0, row.center().y),
        vec2(icon::MENU, icon::MENU),
    );
    let hit = ui.interact(star.expand(4.0), ui.id().with(("open-in-star", id)), Sense::click());
    if visible || hit.hovered() {
        let ink = match (lit, hit.hovered()) {
            (true, _) => p.on_accent,
            (false, true) => p.ink,
            (false, false) => p.ink_3,
        };
        icons::heavy(|| icons::star(ui.painter(), star, ink, starred));
    }
    crate::widgets::pointer_on_hover(ui, &hit);
    hit
}

fn bookmark(p: &Painter, r: Rect, c: Color32) {
    icons::bookmark(p, r, c, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_menu_taller_than_the_window_fits_it_and_scrolls() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(620.0, 420.0));
        // Ten stars: about 610 pt of rows in a 420 pt window.
        let (pos, height) = frame(pos2(300.0, 200.0), 610.0, screen);
        assert_eq!((pos.y, height), (8.0, 404.0));
        // A short menu keeps its height and only moves up to fit.
        assert_eq!(frame(pos2(300.0, 300.0), 330.0, screen), (pos2(300.0, 82.0), 330.0));
        // Arrowing to the last row scrolls it into view, but never past the end.
        let last = 610.0 - PADDING_Y;
        assert_eq!(scroll_to_show(0.0, last - ITEM_HEIGHT, last, 404.0, 610.0), 206.0);
        // Back to the first row scrolls to the top; a visible row leaves the scroll alone.
        assert_eq!(scroll_to_show(206.0, ROWS_TOP, ROWS_TOP + ITEM_HEIGHT, 404.0, 610.0), 0.0);
        assert_eq!(scroll_to_show(40.0, 200.0, 200.0 + ITEM_HEIGHT, 404.0, 610.0), 40.0);
    }

    #[test]
    fn the_submenu_fits_its_labels_and_keeps_the_star_slot() {
        // Room for padding, globe, gap, label and star slot, every time.
        let extra = 2.0 * PADDING_X + 2.0 * tokens::ITEM_PADDING + icon::MENU + tokens::ICON_GAP + STAR_SLOT;
        assert_eq!(open_in_width(130.0), 130.0 + extra);
        // Short labels keep a usable width; very long ones stop at the max (and are cut with "…").
        assert_eq!(open_in_width(10.0), OPEN_IN_MIN_WIDTH);
        assert_eq!(open_in_width(1000.0), OPEN_IN_MAX_WIDTH);
    }

    #[test]
    fn globes_take_only_a_hint_of_the_brand() {
        let gray = Color32::from_rgb(0x6B, 0x6B, 0x6B);
        let red = Color32::from_rgb(0xFF, 0x00, 0x00);
        let hint = mix(gray, red, GLOBE_BRAND_SHARE);
        // Mostly the muted ink: no channel strays more than half the way to the brand colour.
        assert!(hint.r() < 0x6B + (0xFF - 0x6B) / 2 && hint.g() > 0x6B / 2, "{hint:?}");
        assert_eq!(mix(gray, red, 0.0), gray);
        assert_eq!(mix(gray, red, 1.0), red);
    }

    #[test]
    fn stars_persist_in_settings() {
        let mut settings = crate::app::Settings::default();
        open_in::toggle_star(&mut settings.open_in_starred, "logos");
        open_in::toggle_star(&mut settings.open_in_starred, "chatgpt");
        let json = serde_json::to_string(&settings).unwrap();
        let back: crate::app::Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.open_in_starred, ["logos", "chatgpt"]);
        assert_eq!(
            open_in::menu_order(&back.open_in_starred)[..2].iter().map(|a| a.id).collect::<Vec<_>>(),
            ["logos", "chatgpt"]
        );
        // Settings saved before stars existed load with none.
        let old: crate::app::Settings = serde_json::from_str("{\"translation\":\"kjv\"}").unwrap();
        assert!(old.open_in_starred.is_empty());
    }
}
