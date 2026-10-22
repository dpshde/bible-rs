//! The "Open in ›" flyout on the verse menu: static adapters from the vendored links snapshot,
//! grouped like route.bible, with vendored favicons and scroll that matches the reader's feel.

use std::collections::HashMap;

use bible_core::open_in::{Adapter, Catalog, Passage};
use eframe::egui::{
    self, Align2, Color32, ColorImage, Key, Modifiers, Order, Pos2, Rect, Sense, TextureHandle,
    TextureOptions, pos2, vec2,
};

use crate::actions::Action;
use crate::icons;
use crate::menu::{self, ITEM_HEIGHT, PADDING_X, PADDING_Y, SEPARATOR_HEIGHT};
use crate::theme::{Palette, icon, menu as tokens, text};

const WIDTH: f32 = 240.0;
/// How many adapter rows show before the list scrolls (headers don't count toward this).
const VISIBLE: usize = 12;
/// Cap a single frame's wheel/trackpad step so a notch can't leap the whole list.
const MAX_SCROLL_STEP: f32 = ITEM_HEIGHT * 3.0;
/// Muted attribution under the list; clicking it opens route.bible.
pub const FOOTER_LABEL: &str = "Supplied by route.bible";
pub const FOOTER_URL: &str = "https://route.bible";

fn footer_height() -> f32 {
    tokens::HEADER_HEIGHT
}

/// Header label + 1px rule under it (house rule: no other dividers).
fn header_block_height() -> f32 {
    tokens::HEADER_HEIGHT + SEPARATOR_HEIGHT
}

#[derive(Clone, Copy)]
enum Entry<'a> {
    Header(&'static str),
    Item(usize, &'a Adapter),
}

pub struct OpenInMenu {
    /// Top-left of the flyout.
    pub pos: Pos2,
    /// Index into the flat adapter list (headers are not selectable).
    highlighted: Option<usize>,
    /// Filter text (typed while the flyout is open).
    filter: String,
    opened: bool,
    /// Pixel offset into the scrollable content (0 = top).
    scroll_y: f32,
    /// Lazily decoded vendored PNGs, keyed by adapter id.
    textures: HashMap<String, TextureHandle>,
}

impl OpenInMenu {
    pub fn new(pos: Pos2) -> Self {
        Self {
            pos,
            highlighted: Some(0),
            filter: String::new(),
            opened: false,
            scroll_y: 0.0,
            textures: HashMap::new(),
        }
    }

    fn entries<'a>(catalog: &'a Catalog, filter: &str) -> (Vec<Entry<'a>>, Vec<&'a Adapter>) {
        let q = filter.trim().to_ascii_lowercase();
        let mut entries = Vec::new();
        let mut flat = Vec::new();
        for section in catalog.menu_sections() {
            let items: Vec<&Adapter> = section
                .items
                .into_iter()
                .filter(|a| q.is_empty() || a.name.to_ascii_lowercase().contains(&q) || a.id.contains(&q))
                .collect();
            if items.is_empty() {
                continue;
            }
            entries.push(Entry::Header(section.label));
            for a in items {
                let idx = flat.len();
                flat.push(a);
                entries.push(Entry::Item(idx, a));
            }
        }
        (entries, flat)
    }

    fn content_height(entries: &[Entry<'_>]) -> f32 {
        entries
            .iter()
            .map(|e| match e {
                Entry::Header(_) => header_block_height(),
                Entry::Item(_, _) => ITEM_HEIGHT,
            })
            .sum()
    }

    fn max_scroll(entries: &[Entry<'_>], view_h: f32) -> f32 {
        (Self::content_height(entries) - view_h).max(0.0)
    }

    fn adapter_y(entries: &[Entry<'_>], adapter_idx: usize) -> f32 {
        let mut y = 0.0;
        for e in entries {
            match e {
                Entry::Header(_) => y += header_block_height(),
                Entry::Item(i, _) => {
                    if *i == adapter_idx {
                        return y;
                    }
                    y += ITEM_HEIGHT;
                }
            }
        }
        y
    }

    fn ensure_visible(&mut self, adapter_idx: usize, entries: &[Entry<'_>], view_h: f32) {
        let y = Self::adapter_y(entries, adapter_idx);
        if y < self.scroll_y {
            self.scroll_y = y;
        } else if y + ITEM_HEIGHT > self.scroll_y + view_h {
            self.scroll_y = y + ITEM_HEIGHT - view_h;
        }
        self.scroll_y = self.scroll_y.clamp(0.0, Self::max_scroll(entries, view_h));
    }

    fn texture(&mut self, ctx: &egui::Context, id: &str) -> Option<TextureHandle> {
        if let Some(t) = self.textures.get(id) {
            return Some(t.clone());
        }
        let bytes = bible_core::open_in_icons::png(id)?;
        let img = image::load_from_memory(bytes).ok()?.into_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        let tex = ctx.load_texture(format!("open-in-icon-{id}"), color, TextureOptions::LINEAR);
        self.textures.insert(id.to_string(), tex.clone());
        Some(tex)
    }

    /// Returns `true` when the flyout should close (and the parent verse menu with it on pick).
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        catalog: &Catalog,
        actions: &mut Vec<Action>,
    ) -> bool {
        let prev_filter = self.filter.clone();
        let (down, up, enter, esc) = ctx.input_mut(|i| {
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
        let (entries, rows) = Self::entries(catalog, &self.filter);
        let view_h = ITEM_HEIGHT * VISIBLE.min(rows.len().max(1)) as f32;
        if self.filter != prev_filter {
            self.scroll_y = 0.0;
            self.highlighted = if rows.is_empty() { None } else { Some(0) };
        }
        if esc {
            return true;
        }
        if rows.is_empty() {
            self.highlighted = None;
            self.scroll_y = 0.0;
        } else if let Some(h) = self.highlighted {
            if h >= rows.len() {
                self.highlighted = Some(0);
            }
        } else {
            self.highlighted = Some(0);
        }
        if down && !rows.is_empty() {
            let at = self.highlighted.unwrap_or(0);
            let next = (at + 1) % rows.len();
            self.highlighted = Some(next);
            self.ensure_visible(next, &entries, view_h);
        }
        if up && !rows.is_empty() {
            let at = self.highlighted.unwrap_or(0);
            let next = (at + rows.len() - 1) % rows.len();
            self.highlighted = Some(next);
            self.ensure_visible(next, &entries, view_h);
        }
        if enter
            && let Some(i) = self.highlighted
            && let Some(a) = rows.get(i)
        {
            actions.push(Action::OpenIn(a.id.clone()));
            return true;
        }

        let list_h = if rows.is_empty() { ITEM_HEIGHT } else { view_h };
        let filter_h = if self.filter.is_empty() { 0.0 } else { tokens::HEADER_HEIGHT };
        let height = PADDING_Y + filter_h + list_h + footer_height() + PADDING_Y;
        let screen = ctx.content_rect();
        let pos = pos2(
            self.pos.x.min(screen.right() - WIDTH - 8.0).max(screen.left() + 8.0),
            self.pos.y.min(screen.bottom() - height - 8.0).max(screen.top() + 8.0),
        );
        let mut close = false;
        let max_scroll = Self::max_scroll(&entries, list_h);
        self.scroll_y = self.scroll_y.clamp(0.0, max_scroll);

        // Warm textures for adapters near the scroll window.
        let warm_lo = Self::adapter_y(&entries, 0);
        let _ = warm_lo;
        for e in &entries {
            if let Entry::Item(i, a) = e {
                let y = Self::adapter_y(&entries, *i);
                if y + ITEM_HEIGHT >= self.scroll_y - ITEM_HEIGHT && y <= self.scroll_y + list_h + ITEM_HEIGHT
                {
                    let _ = self.texture(ctx, &a.id);
                }
            }
        }

        let area = egui::Area::new(egui::Id::new("open-in-menu"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(pos, vec2(WIDTH, height));
                p.shadows.popover.paint(ui.painter(), rect, tokens::RADIUS);
                ui.painter().rect_filled(rect, tokens::RADIUS, p.surface);
                let resp = ui.allocate_rect(rect, Sense::hover());
                if resp.hovered() || resp.contains_pointer() {
                    let dy = ui.ctx().input_mut(|i| {
                        let d = i.smooth_scroll_delta.y;
                        i.smooth_scroll_delta.y = 0.0;
                        d
                    });
                    let step = dy.clamp(-MAX_SCROLL_STEP, MAX_SCROLL_STEP);
                    if step != 0.0 {
                        self.scroll_y = (self.scroll_y - step).clamp(0.0, max_scroll);
                    }
                }
                let (left, right) = (rect.left() + PADDING_X, rect.right() - PADDING_X);
                let footer_top = rect.bottom() - PADDING_Y - footer_height();
                let footer = Rect::from_x_y_ranges(left..=right, footer_top..=footer_top + footer_height());
                let footer_hit = ui.interact(footer, ui.id().with("open-in-footer"), Sense::click());
                paint_footer(ui.painter(), p, footer, footer_hit.hovered());
                if footer_hit.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if footer_hit.clicked() {
                    crate::app::open_url(FOOTER_URL);
                    close = true;
                }
                let mut y = rect.top() + PADDING_Y;
                if !self.filter.is_empty() {
                    let header = Rect::from_x_y_ranges(left..=right, y..=y + tokens::HEADER_HEIGHT);
                    menu::paint_header(ui.painter(), p, header, &format!("“{}”", self.filter));
                    y += tokens::HEADER_HEIGHT;
                }
                if rows.is_empty() {
                    let r = Rect::from_x_y_ranges(left..=right, y..=y + ITEM_HEIGHT);
                    menu::paint_header(ui.painter(), p, r, "No matching apps");
                    return;
                }
                let list = Rect::from_x_y_ranges(left..=right, y..=y + list_h);
                let painter = ui.painter().with_clip_rect(list);
                let mut row_y = list.top() - self.scroll_y;
                for entry in &entries {
                    match entry {
                        Entry::Header(label) => {
                            let h = header_block_height();
                            if row_y + h >= list.top() && row_y < list.bottom() {
                                let header = Rect::from_x_y_ranges(
                                    left..=right,
                                    row_y..=row_y + tokens::HEADER_HEIGHT,
                                );
                                paint_section_header(&painter, p, header, label);
                                menu::paint_separator(
                                    &painter,
                                    p,
                                    left,
                                    right,
                                    row_y + tokens::HEADER_HEIGHT,
                                );
                            }
                            row_y += h;
                        }
                        Entry::Item(i, adapter) => {
                            if row_y >= list.bottom() {
                                break;
                            }
                            if row_y + ITEM_HEIGHT > list.top() {
                                let r = Rect::from_x_y_ranges(left..=right, row_y..=row_y + ITEM_HEIGHT);
                                let hit = ui.interact(
                                    r.intersect(list),
                                    ui.id().with(("open-in", &adapter.id)),
                                    Sense::click(),
                                );
                                if hit.hovered() && ui.input(|inp| inp.pointer.delta() != egui::Vec2::ZERO) {
                                    self.highlighted = Some(*i);
                                }
                                let lit = self.highlighted == Some(*i);
                                let tex = self.textures.get(&adapter.id);
                                paint_adapter_row(
                                    &painter,
                                    p,
                                    r,
                                    adapter.name.as_str(),
                                    tex,
                                    lit,
                                    hit.is_pointer_button_down_on(),
                                );
                                if hit.clicked() {
                                    actions.push(Action::OpenIn(adapter.id.clone()));
                                    close = true;
                                }
                            }
                            row_y += ITEM_HEIGHT;
                        }
                    }
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

/// Small muted uppercase-ish section label (not clickable).
fn paint_section_header(painter: &egui::Painter, p: &Palette, r: Rect, label: &str) {
    let shown = label.to_ascii_uppercase();
    painter.text(
        pos2(r.left() + tokens::ITEM_PADDING, r.center().y),
        Align2::LEFT_CENTER,
        shown,
        text::CAPTION.font(),
        p.ink_3,
    );
}

/// Small muted attribution line; slightly stronger ink on hover.
fn paint_footer(painter: &egui::Painter, p: &Palette, r: Rect, hovered: bool) {
    painter.text(
        pos2(r.left() + tokens::ITEM_PADDING, r.center().y),
        Align2::LEFT_CENTER,
        FOOTER_LABEL,
        text::CAPTION.font(),
        if hovered { p.ink_2 } else { p.ink_3 },
    );
}

fn paint_adapter_row(
    painter: &egui::Painter,
    p: &Palette,
    r: Rect,
    label: &str,
    tex: Option<&TextureHandle>,
    lit: bool,
    pressed: bool,
) {
    if lit {
        painter.rect_filled(r, tokens::ITEM_RADIUS, p.accent);
        if pressed {
            painter.rect_filled(r, tokens::ITEM_RADIUS, Color32::from_black_alpha(18));
        }
    }
    let (ink, ink2) = match lit {
        true => (p.on_accent, p.on_accent),
        false => (p.ink, p.ink_2),
    };
    let cy = r.center().y;
    let x = menu::text_left(r);
    let glyph = Rect::from_min_size(pos2(x, cy - icon::MENU / 2.0), vec2(icon::MENU, icon::MENU));
    if let Some(tex) = tex {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(tex.id(), glyph, uv, Color32::WHITE);
    } else {
        crate::icons::heavy(|| icons::globe(painter, glyph, ink2));
    }
    painter.text(
        pos2(glyph.right() + tokens::ICON_GAP, cy),
        Align2::LEFT_CENTER,
        label,
        text::UI.font(),
        ink,
    );
}

/// Label for the parent row; callers pass whether the flyout is open.
pub fn parent_label() -> &'static str {
    "Open in"
}

pub fn passage_from_selection(refs: &[bible_core::VerseRef]) -> Option<Passage> {
    Passage::from_refs(refs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_credits_route_bible() {
        assert_eq!(FOOTER_LABEL, "Supplied by route.bible");
        assert_eq!(FOOTER_URL, "https://route.bible");
        assert!(footer_height() > 0.0);
    }
}
