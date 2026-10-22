//! The "Open in ›" flyout on the verse menu: static adapters from the vendored links snapshot,
//! grouped like route.bible, with vendored favicons and scroll that matches the reader's feel.

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
/// Muted attribution, the last row of the scrolling list; clicking it opens route.bible.
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
}

impl OpenInMenu {
    pub fn new(pos: Pos2) -> Self {
        Self { pos, highlighted: Some(0), filter: String::new(), opened: false, scroll_y: 0.0 }
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
            .sum::<f32>()
            + footer_height()
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

    /// Returns `true` when the flyout should close (and the parent verse menu with it on pick).
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        p: &Palette,
        catalog: &Catalog,
        starred: &[String],
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
        let view_h = (ITEM_HEIGHT * VISIBLE as f32).min(Self::content_height(&entries));
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
        let height = PADDING_Y + filter_h + list_h + PADDING_Y;
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
                    let _ = adapter_icon(ctx, &a.id);
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
                                let tex = adapter_icon(ctx, &adapter.id);
                                paint_adapter_row(
                                    &painter,
                                    p,
                                    r,
                                    adapter.name.as_str(),
                                    tex.as_ref(),
                                    lit,
                                    hit.is_pointer_button_down_on(),
                                );
                                // Star toggle at the trailing edge: shown on hover, kept when starred.
                                let is_starred = starred.contains(&adapter.id);
                                let star_hit = star_toggle(
                                    ui,
                                    &painter,
                                    p,
                                    r,
                                    list,
                                    &adapter.id,
                                    is_starred,
                                    is_starred || hit.hovered(),
                                    lit,
                                );
                                if star_hit.clicked() {
                                    actions.push(Action::ToggleOpenInStar(adapter.id.clone()));
                                } else if hit.clicked() {
                                    actions.push(Action::OpenIn(adapter.id.clone()));
                                    close = true;
                                }
                            }
                            row_y += ITEM_HEIGHT;
                        }
                    }
                }
                // The credit is the list's last row: it scrolls with the adapters.
                let footer = Rect::from_x_y_ranges(left..=right, row_y..=row_y + footer_height());
                if footer.intersects(list) {
                    let footer_hit =
                        ui.interact(footer.intersect(list), ui.id().with("open-in-footer"), Sense::click());
                    paint_footer(&painter, p, footer, footer_hit.hovered());
                    if footer_hit.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if footer_hit.clicked() {
                        crate::app::open_url(FOOTER_URL);
                        close = true;
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

/// An adapter's decoded favicon plus what's needed to keep it legible on any theme.
#[derive(Clone)]
pub struct AdapterIcon {
    tex: TextureHandle,
    /// A bare logo or wordmark (route.bible `fit: "contain"`): drawn whole, no rounding.
    /// Otherwise an app tile, rounded like route.bible's.
    contain: bool,
    stats: IconStats,
}

/// Rounded corners for app tiles: route.bible's 7 px on a 24 px tile.
const TILE_RADIUS: f32 = icon::MENU * 7.0 / 24.0;
/// How far a backing tile reaches past the icon.
const BACKING_PAD: f32 = 2.0;
/// Below this, an icon doesn't read against the background (see [`IconStats::visibility_on`]).
const MIN_VISIBILITY: f32 = 0.03;
/// A pixel "reads" when its alpha-weighted luminance differs from the background by this much.
const INK_DELTA: f32 = 0.2;
/// Background luminances the stats are sampled at: 0, 1/STEPS, ..., 1.
const STEPS: usize = 50;

fn luminance(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}

fn color_luminance(c: Color32) -> f32 {
    luminance(c.r(), c.g(), c.b())
}

/// An icon's luminance and coverage, measured once at decode so the per-frame theme check is free.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconStats {
    /// Share of the icon's square that stands out over a background of luminance `i / STEPS`.
    visibility: [f32; STEPS + 1],
    /// Alpha-weighted mean luminance of the icon's own pixels.
    mean_lum: f32,
}

impl IconStats {
    fn of(img: &image::RgbaImage) -> Self {
        let n = (img.width() * img.height()).max(1) as f32;
        let mut visibility = [0.0; STEPS + 1];
        let (mut lum_sum, mut alpha_sum) = (0.0, 0.0);
        for px in img.pixels() {
            let a = px[3] as f32 / 255.0;
            let l = luminance(px[0], px[1], px[2]);
            lum_sum += l * a;
            alpha_sum += a;
            for (i, v) in visibility.iter_mut().enumerate() {
                if a * (l - i as f32 / STEPS as f32).abs() > INK_DELTA {
                    *v += 1.0;
                }
            }
        }
        visibility.iter_mut().for_each(|v| *v /= n);
        let mean_lum = if alpha_sum > 0.0 { lum_sum / alpha_sum } else { 0.5 };
        Self { visibility, mean_lum }
    }

    /// Visibility over a background of luminance `bg` (interpolated).
    fn visibility_on(&self, bg: f32) -> f32 {
        let x = bg.clamp(0.0, 1.0) * STEPS as f32;
        let i = (x.floor() as usize).min(STEPS - 1);
        let t = x - i as f32;
        self.visibility[i] * (1.0 - t) + self.visibility[i + 1] * t
    }
}

/// The backing tile an icon needs on a background of `bg`, or `None` when it already reads.
/// Light logos get a dark tile, dark logos a light one, so the tile follows the theme.
fn backing_tile(stats: &IconStats, bg: Color32) -> Option<Color32> {
    if stats.visibility_on(color_luminance(bg)) >= MIN_VISIBILITY {
        return None;
    }
    Some(if stats.mean_lum > 0.5 {
        Color32::from_rgb(0x3A, 0x3A, 0x3E)
    } else {
        Color32::from_rgb(0xEC, 0xEC, 0xEE)
    })
}

/// `id`'s icon, decoded once and kept in the context's memory (shared by the flyout and the
/// starred rows in the verse menu).
pub fn adapter_icon(ctx: &egui::Context, id: &str) -> Option<AdapterIcon> {
    let key = egui::Id::new(("open-in-icon", id));
    if let Some(t) = ctx.data(|d| d.get_temp::<AdapterIcon>(key)) {
        return Some(t);
    }
    let bytes = bible_core::open_in_icons::png(id)?;
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let tex = ctx.load_texture(format!("open-in-icon-{id}"), color, TextureOptions::LINEAR);
    let icon = AdapterIcon {
        tex,
        contain: bible_core::open_in_icons::style(id).contain,
        stats: IconStats::of(&img),
    };
    ctx.data_mut(|d| d.insert_temp(key, icon.clone()));
    Some(icon)
}

/// Paints `icon` in `glyph` over `bg` (the menu surface, or the accent on a lit row), backing
/// it with a contrasting tile only if it would otherwise disappear on this theme.
fn paint_icon(painter: &egui::Painter, glyph: Rect, icon: &AdapterIcon, bg: Color32) {
    if let Some(tile) = backing_tile(&icon.stats, bg) {
        painter.rect_filled(glyph.expand(BACKING_PAD), TILE_RADIUS + BACKING_PAD, tile);
    }
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    if icon.contain {
        painter.image(icon.tex.id(), glyph, uv, Color32::WHITE);
    } else {
        painter.add(
            egui::epaint::RectShape::filled(glyph, TILE_RADIUS, Color32::WHITE)
                .with_texture(icon.tex.id(), uv),
        );
    }
}

/// The star at a row's trailing edge (submenu rows and the verse menu's starred rows). Drawn
/// filled when `starred`, and only when `visible` or itself hovered; returns its click response.
#[allow(clippy::too_many_arguments)]
pub(crate) fn star_toggle(
    ui: &egui::Ui,
    painter: &egui::Painter,
    p: &Palette,
    row: Rect,
    clip: Rect,
    id: &str,
    starred: bool,
    visible: bool,
    lit: bool,
) -> egui::Response {
    let star = Rect::from_center_size(
        pos2(row.right() - tokens::ITEM_PADDING - icon::MENU / 2.0, row.center().y),
        vec2(icon::MENU, icon::MENU),
    );
    let hit =
        ui.interact(star.expand(4.0).intersect(clip), ui.id().with(("open-in-star", id)), Sense::click());
    if visible || hit.hovered() {
        let ink = match (lit, hit.hovered()) {
            (true, _) => p.on_accent,
            (false, true) => p.ink,
            (false, false) => p.ink_3,
        };
        icons::heavy(|| icons::star(painter, star, ink, starred));
    }
    if hit.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    hit
}

/// Stars `id`, or unstars it when it already is. New stars go last.
pub fn toggle_star(starred: &mut Vec<String>, id: &str) {
    if let Some(i) = starred.iter().position(|s| s == id) {
        starred.remove(i);
    } else {
        starred.push(id.to_string());
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

pub(crate) fn paint_adapter_row(
    painter: &egui::Painter,
    p: &Palette,
    r: Rect,
    label: &str,
    tex: Option<&AdapterIcon>,
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
    if let Some(icon) = tex {
        paint_icon(painter, glyph, icon, if lit { p.accent } else { p.surface });
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

    fn stats(id: &str) -> IconStats {
        let png = bible_core::open_in_icons::png(id).unwrap();
        IconStats::of(&image::load_from_memory(png).unwrap().into_rgba8())
    }

    const LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    const SEPIA: Color32 = Color32::from_rgb(0xFB, 0xF5, 0xE8);
    const DARK: Color32 = Color32::from_rgb(0x26, 0x26, 0x2A);

    #[test]
    fn backing_tile_follows_the_theme() {
        // Bible.is is a white wordmark: dark tile on light/sepia, none on dark.
        let bible_is = stats("bible-is");
        let tile = backing_tile(&bible_is, LIGHT).expect("white logo on white needs a tile");
        assert!(color_luminance(tile) < 0.3);
        assert!(backing_tile(&bible_is, SEPIA).is_some());
        assert_eq!(backing_tile(&bible_is, DARK), None);
        // A near-black tile (as Kindled's was): light backing on dark, none on light.
        let kindled =
            IconStats::of(&image::RgbaImage::from_pixel(64, 64, image::Rgba([0x2A, 0x1F, 0x10, 0xFF])));
        let tile = backing_tile(&kindled, DARK).expect("dark tile on dark needs a backing");
        assert!(color_luminance(tile) > 0.7);
        assert_eq!(backing_tile(&kindled, LIGHT), None);
        // Icons that already read get nothing in either theme.
        for id in ["bible-app", "bible-gateway", "logos"] {
            assert_eq!(backing_tile(&stats(id), LIGHT), None, "{id} light");
            assert_eq!(backing_tile(&stats(id), DARK), None, "{id} dark");
        }
    }

    #[test]
    fn every_icon_is_legible_on_every_theme() {
        let catalog = bible_core::open_in::Catalog::bundled();
        let mut checked = 0;
        for a in &catalog.items {
            if bible_core::open_in_icons::png(&a.id).is_none() {
                continue;
            }
            let s = stats(&a.id);
            for bg in [LIGHT, SEPIA, DARK] {
                let bg_lum = color_luminance(bg);
                match backing_tile(&s, bg) {
                    None => assert!(s.visibility_on(bg_lum) >= MIN_VISIBILITY),
                    Some(tile) => {
                        let tile_lum = color_luminance(tile);
                        assert!((tile_lum - bg_lum).abs() > 0.5, "{}: tile must stand off the menu", a.id);
                        assert!(
                            s.visibility_on(tile_lum) >= MIN_VISIBILITY,
                            "{} unreadable on its tile ({bg:?})",
                            a.id
                        );
                    }
                }
            }
            checked += 1;
        }
        assert!(checked >= 60, "only {checked} icons checked");
    }

    #[test]
    fn toggle_star_adds_last_and_removes() {
        let mut starred = Vec::new();
        toggle_star(&mut starred, "wedevote");
        toggle_star(&mut starred, "bible-gateway");
        assert_eq!(starred, ["wedevote", "bible-gateway"]);
        toggle_star(&mut starred, "wedevote");
        assert_eq!(starred, ["bible-gateway"]);
    }

    #[test]
    fn stars_persist_in_settings() {
        let mut settings = crate::app::Settings::default();
        toggle_star(&mut settings.open_in_starred, "wedevote");
        let json = serde_json::to_string(&settings).unwrap();
        let back: crate::app::Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.open_in_starred, ["wedevote"]);
        // Settings saved before stars existed load with none.
        let old: crate::app::Settings = serde_json::from_str("{\"translation\":\"kjv\"}").unwrap();
        assert!(old.open_in_starred.is_empty());
    }

    #[test]
    fn footer_credits_route_bible() {
        assert_eq!(FOOTER_LABEL, "Supplied by route.bible");
        assert_eq!(FOOTER_URL, "https://route.bible");
        assert!(footer_height() > 0.0);
    }

    #[test]
    fn footer_is_the_last_row_of_the_scrolling_list() {
        let catalog = Catalog::bundled();
        let (entries, rows) = OpenInMenu::entries(&catalog, "");
        let rows_h: f32 = entries
            .iter()
            .map(|e| match e {
                Entry::Header(_) => header_block_height(),
                Entry::Item(..) => ITEM_HEIGHT,
            })
            .sum();
        assert!(rows.len() > VISIBLE);
        assert_eq!(OpenInMenu::content_height(&entries), rows_h + footer_height());
        // Scrolled to the end, the footer is what fills the bottom of the list.
        let view_h = ITEM_HEIGHT * VISIBLE as f32;
        assert_eq!(OpenInMenu::max_scroll(&entries, view_h) + view_h, rows_h + footer_height());
    }
}
