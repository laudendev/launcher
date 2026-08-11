// Visual theme matching softstore's style.css design tokens as closely
// as egui allows without shipping actual font files. Fonts are egui's
// defaults (a clean sans, not Space Grotesk/Inter/JetBrains Mono) —
// swap in real .ttf/.otf files later via egui::FontDefinitions if you
// want an exact typographic match; everything else (color, radius,
// spacing, hover behavior) is a direct translation of the CSS values.
//
// Source values, from style.css:
//   --brand-blue      -> accent, primary action color
//   --card-bg         -> panel/window background
//   --border          -> stroke color on cards, inputs
//   --radius / -sm     -> corner rounding
//   --ink-soft         -> secondary/muted text
// Dark-mode block (--brand-blue -> cyan, neon glow) is applied when
// the OS reports a dark preference, same trigger as the CSS media query.

use eframe::egui;
use egui::{Color32, Rounding, Stroke};

// Light mode (CSS defaults, no @media override).
const LIGHT_BG: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFC);
const LIGHT_CARD_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
const LIGHT_BORDER: Color32 = Color32::from_rgb(0xE2, 0xE2, 0xEA);
const LIGHT_INK: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1F);
const LIGHT_INK_SOFT: Color32 = Color32::from_rgb(0x6B, 0x6B, 0x76);
const BRAND_BLUE: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6); // --brand-blue

// Dark mode (--brand-blue becomes cyan per the CSS's dark block; the
// glow color used there, rgba(0,240,255,.25), is that same cyan).
const DARK_BG: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0F);
const DARK_CARD_BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x1C);
const DARK_BORDER: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x36);
const DARK_INK: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF5);
const DARK_INK_SOFT: Color32 = Color32::from_rgb(0x9A, 0x9A, 0xA8);
const DARK_ACCENT: Color32 = Color32::from_rgb(0x00, 0xF0, 0xFF); // dark-mode brand-blue -> cyan
const DARK_BUTTON_TEXT: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0F); // matches CSS .launcher-button dark color

const RADIUS: u8 = 10; // --radius
const RADIUS_SM: u8 = 6; // --radius-sm

/// Applies the theme to `ctx`. Call once, e.g. from `run_native`'s setup
/// closure or the top of the first frame. Picks light/dark based on the
/// OS preference eframe reports, mirroring the CSS's
/// `@media (prefers-color-scheme: dark)` block.
pub fn apply(ctx: &egui::Context) {
    let dark = ctx.style().visuals.dark_mode;
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    let (bg, card_bg, border, ink, ink_soft, accent, button_text) = if dark {
        (
            DARK_BG,
            DARK_CARD_BG,
            DARK_BORDER,
            DARK_INK,
            DARK_INK_SOFT,
            DARK_ACCENT,
            DARK_BUTTON_TEXT,
        )
    } else {
        (
            LIGHT_BG,
            LIGHT_CARD_BG,
            LIGHT_BORDER,
            LIGHT_INK,
            LIGHT_INK_SOFT,
            BRAND_BLUE,
            Color32::WHITE,
        )
    };

    v.panel_fill = bg;
    v.window_fill = card_bg;
    v.extreme_bg_color = card_bg; // text-edit background
    v.faint_bg_color = card_bg;

    v.window_stroke = Stroke::new(1.0_f32, border);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent);
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, accent);

    v.override_text_color = Some(ink);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, ink_soft);

    // Primary button (Download): dimmed accent at rest rather than the
    // raw, fully-saturated color — the un-dimmed accent read as too
    // bright/alert-colored at rest, and hover was compounding that by
    // lightening an already-bright base further.
    let button_rest = darken(accent, 0.30);
    v.widgets.inactive.weak_bg_fill = button_rest;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, button_text);
    v.widgets.hovered.weak_bg_fill = lighten(button_rest, 0.08);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, button_text);
    v.widgets.active.weak_bg_fill = darken(button_rest, 0.10);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, button_text);

    // Disabled state: matches an unfilled/greyed button rather than the
    // accent color, so "lights up" (main.rs) is visually obvious.
    v.widgets.noninteractive.weak_bg_fill = card_bg;

    // This eframe version's Visuals still use the older `rounding: Rounding`
    // field name (pre corner_radius rename) on both Visuals and
    // WidgetVisuals — no top-level window_corner_radius exists here, so
    // window rounding is set via window_rounding instead.
    let radius = Rounding::same(RADIUS as f32);
    let radius_sm = Rounding::same(RADIUS_SM as f32);
    v.window_rounding = radius;
    v.widgets.noninteractive.rounding = radius;
    v.widgets.inactive.rounding = radius_sm;
    v.widgets.hovered.rounding = radius_sm;
    v.widgets.active.rounding = radius_sm;

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(20.0, 10.0);

    ctx.set_style(style);
}

fn lighten(c: Color32, t: f32) -> Color32 {
    blend(c, Color32::WHITE, t)
}

fn darken(c: Color32, t: f32) -> Color32 {
    blend(c, Color32::BLACK, t)
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}
