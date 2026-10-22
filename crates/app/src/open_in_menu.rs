//! The "Open in ›" flyout on the verse menu: static adapters from the vendored links snapshot.

use bible_core::open_in::{Adapter, Catalog, Passage};
use eframe::egui::{self, Key, Modifiers, Order, Pos2, Rect, Sense, pos2, vec2};

use crate::actions::Action;
use crate::icons;
use crate::menu::{self, ITEM_HEIGHT, PADDING_X, PADDING_Y};
use crate::theme::{Palette, menu as tokens};
use crate::widgets::IconFn;

const WIDTH: f32 = 240.0;
/// How many adapters show before the list scrolls.
const VISIBLE: usize = 12;

pub struct OpenInMenu {
    /// Top-left of the flyout.
    pub pos: Pos2,
    highlighted: Option<usize>,
    /// Filter text (typed while the flyout is open).
    filter: String,
    opened: bool,
}

impl OpenInMenu {
    pub fn new(pos: Pos2) -> Self {
        Self { pos, highlighted: Some(0), filter: String::new(), opened: false }
    }

    fn rows<'a>(catalog: &'a Catalog, filter: &str) -> Vec<&'a Adapter> {
        let q = filter.trim().to_ascii_lowercase();
        catalog
            .menu_items()
            .into_iter()
            .filter(|a| q.is_empty() || a.name.to_ascii_lowercase().contains(&q) || a.id.contains(&q))
            .collect()
    }

    /// Returns `true` when the flyout should close (and the parent verse menu with it on pick).
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        catalog: &Catalog,
        actions: &mut Vec<Action>,
    ) -> bool {
        let rows = Self::rows(catalog, &self.filter);
        let (down, up, enter, esc) = ctx.input_mut(|i| {
            // Typed characters filter the list (letters/digits/space/backspace).
            for ev in &i.events {
                if let egui::Event::Text(t) = ev {
                    for c in t.chars() {
                        if !c.is_control() {
                            self.filter.push(c);
                        }
                    }
                }
            }
            if i.consume_key(Modifiers::NONE, Key::Backspace) {
                self.filter.pop();
            }
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if esc {
            return true;
        }
        if rows.is_empty() {
            self.highlighted = None;
        } else if let Some(h) = self.highlighted {
            if h >= rows.len() {
                self.highlighted = Some(0);
            }
        } else {
            self.highlighted = Some(0);
        }
        if down && !rows.is_empty() {
            let at = self.highlighted.unwrap_or(0);
            self.highlighted = Some((at + 1) % rows.len());
        }
        if up && !rows.is_empty() {
            let at = self.highlighted.unwrap_or(0);
            self.highlighted = Some((at + rows.len() - 1) % rows.len());
        }
        if enter
            && let Some(i) = self.highlighted
            && let Some(a) = rows.get(i)
        {
            actions.push(Action::OpenIn(a.id.clone()));
            return true;
        }

        let shown = rows.len().min(VISIBLE);
        let list_h = ITEM_HEIGHT * shown as f32;
        let filter_h = if self.filter.is_empty() { 0.0 } else { tokens::HEADER_HEIGHT };
        let height = PADDING_Y + filter_h + list_h + PADDING_Y;
        let screen = ctx.content_rect();
        let pos = pos2(
            self.pos.x.min(screen.right() - WIDTH - 8.0).max(screen.left() + 8.0),
            self.pos.y.min(screen.bottom() - height - 8.0).max(screen.top() + 8.0),
        );
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("open-in-menu"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(pos, vec2(WIDTH, height));
                p.shadows.popover.paint(ui.painter(), rect, tokens::RADIUS);
                ui.painter().rect_filled(rect, tokens::RADIUS, p.surface);
                let (_, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                let (left, right) = (rect.left() + PADDING_X, rect.right() - PADDING_X);
                let mut y = rect.top() + PADDING_Y;
                if !self.filter.is_empty() {
                    let header = Rect::from_x_y_ranges(left..=right, y..=y + tokens::HEADER_HEIGHT);
                    menu::paint_header(ui.painter(), p, header, &format!("“{}”", self.filter));
                    y += tokens::HEADER_HEIGHT;
                }
                // Scroll window around the highlighted row.
                let start = match self.highlighted {
                    Some(h) if rows.len() > VISIBLE => {
                        h.saturating_sub(VISIBLE / 2).min(rows.len() - VISIBLE)
                    }
                    _ => 0,
                };
                for (k, adapter) in rows.iter().enumerate().skip(start).take(VISIBLE) {
                    let i = k;
                    let r = Rect::from_x_y_ranges(left..=right, y..=y + ITEM_HEIGHT);
                    let resp = ui.interact(r, ui.id().with(("open-in", &adapter.id)), Sense::click());
                    if resp.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) {
                        self.highlighted = Some(i);
                    }
                    let row = menu::Row {
                        icon: icon_for(adapter),
                        label: &adapter.name,
                        shortcut: "",
                        enabled: true,
                        lit: self.highlighted == Some(i),
                        pressed: resp.is_pointer_button_down_on(),
                    };
                    menu::paint_row(ui.painter(), p, r, row);
                    if resp.clicked() {
                        actions.push(Action::OpenIn(adapter.id.clone()));
                        close = true;
                    }
                    y += ITEM_HEIGHT;
                }
                if rows.is_empty() {
                    let r = Rect::from_x_y_ranges(left..=right, y..=y + ITEM_HEIGHT);
                    menu::paint_header(ui.painter(), p, r, "No matching apps");
                }
            });
        if !self.opened {
            self.opened = true;
        } else if area.response.clicked_elsewhere() {
            close = true;
        }
        close
    }
}

fn icon_for(a: &Adapter) -> IconFn {
    match a.catalog_type.as_deref() {
        Some("explore") => icons::compare,
        Some("route") => icons::note,
        _ => icons::globe,
    }
}

/// Label for the parent row; callers pass whether the flyout is open.
pub fn parent_label() -> &'static str {
    "Open in"
}

pub fn passage_from_selection(refs: &[bible_core::VerseRef]) -> Option<Passage> {
    Passage::from_refs(refs)
}
