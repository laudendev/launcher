// Goo flask animation, reusable as an embeddable egui widget.
//
// Same CPU filter pipeline as the standalone flask_goo demo (blur ->
// feColorMatrix alpha threshold -> clip), refactored so it can be
// dropped into any panel instead of owning the whole window. Call
// `GooWidget::default()` once, keep it around in your App state, and
// call `.show(ui, size_pts)` each frame you want it visible.

use eframe::egui;
use egui::{ColorImage, TextureHandle, TextureOptions, Vec2};

const VB: f32 = 135.46667;

const BG: [f32; 3] = [0x3B as f32 / 255.0, 0x82 as f32 / 255.0, 0xF6 as f32 / 255.0];
const DARK: [f32; 3] = [0x1E as f32 / 255.0, 0x3A as f32 / 255.0, 0x8A as f32 / 255.0];
const GRAD_IN: [f32; 3] = [1.0, 1.0, 1.0];
const GRAD_OUT: [f32; 3] = [0xBF as f32 / 255.0, 0xDB as f32 / 255.0, 0xFE as f32 / 255.0];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

#[derive(Default)]
pub struct GooWidget {
    tex: Option<TextureHandle>,
    statics: Option<Statics>,
}

impl GooWidget {
    /// Paints the animation at `size_pts` square and requests continuous
    /// repaint while visible. Call this every frame you want it shown;
    /// stop calling it (e.g. once download completes) and it simply
    /// stops costing anything — no background timer to cancel.
    pub fn show(&mut self, ui: &mut egui::Ui, size_pts: f32) {
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let px = ((size_pts * ppp).round() as usize).max(16);
        if self.statics.as_ref().map(|s| s.px) != Some(px) {
            self.statics = Some(Statics::build(px));
        }
        let st = self.statics.as_mut().unwrap();

        let t = ctx.input(|i| i.time);
        st.render_goo(t);
        st.compose();

        let img = ColorImage::from_rgba_unmultiplied([px, px], &st.rgba);
        match &mut self.tex {
            Some(tex) => tex.set(img, TextureOptions::LINEAR),
            None => self.tex = Some(ctx.load_texture("goo_widget", img, TextureOptions::LINEAR)),
        }

        if let Some(tex) = &self.tex {
            ui.image(egui::load::SizedTexture::new(
                tex.id(),
                Vec2::splat(size_pts),
            ));
        }
        ctx.request_repaint(); // only while this widget is actually shown
    }
}

// --- everything below is unchanged from flask_goo's implementation ---

struct Statics {
    px: usize,
    scale: f32,
    under: Vec<[f32; 4]>,
    clip: Vec<f32>,
    stroke: Vec<f32>,
    gx0: usize,
    gy0: usize,
    gw: usize,
    gh: usize,
    kernel: Vec<f32>,
    krad: i32,
    goo: Vec<[f32; 4]>,
    tmp: Vec<[f32; 4]>,
    rgba: Vec<u8>,
}

#[inline]
fn coverage(sd_units: f32, scale: f32) -> f32 {
    (0.5 - sd_units * scale).clamp(0.0, 1.0)
}

#[inline]
fn over(dst: &mut [f32; 4], src_rgb: [f32; 3], src_a: f32) {
    if src_a <= 0.0 {
        return;
    }
    let out_a = src_a + dst[3] * (1.0 - src_a);
    if out_a > 0.0 {
        for c in 0..3 {
            dst[c] = (src_rgb[c] * src_a + dst[c] * dst[3] * (1.0 - src_a)) / out_a;
        }
    }
    dst[3] = out_a;
}

#[inline]
fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py) = (p.0 - a.0, p.1 - a.1);
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let len2 = ex * ex + ey * ey;
    let t = if len2 > 0.0 {
        ((px * ex + py * ey) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (dx, dy) = (px - t * ex, py - t * ey);
    (dx * dx + dy * dy).sqrt()
}

#[inline]
fn rounded_rect_sd(p: (f32, f32)) -> f32 {
    let r = 20.0;
    let half = VB / 2.0;
    let qx = (p.0 - half).abs() - (half - r);
    let qy = (p.1 - half).abs() - (half - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

fn quad(out: &mut Vec<(f32, f32)>, p0: (f32, f32), c: (f32, f32), p1: (f32, f32)) {
    const N: usize = 6;
    for i in 1..=N {
        let t = i as f32 / N as f32;
        let u = 1.0 - t;
        out.push((
            u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0,
            u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1,
        ));
    }
}

fn flask_outline() -> Vec<(f32, f32)> {
    let mut p: Vec<(f32, f32)> = vec![(60.0, 20.0)];
    quad(&mut p, (60.0, 20.0), (59.0, 20.0), (59.0, 22.0));
    p.push((50.0, 86.0));
    p.push((58.0, 98.0));
    p.push((49.0, 114.0));
    quad(&mut p, (49.0, 114.0), (48.0, 116.0), (51.0, 116.0));
    p.push((84.0, 116.0));
    quad(&mut p, (84.0, 116.0), (87.0, 116.0), (86.0, 114.0));
    p.push((77.0, 98.0));
    p.push((85.0, 86.0));
    p.push((76.0, 22.0));
    quad(&mut p, (76.0, 22.0), (76.0, 20.0), (75.0, 20.0));
    p
}

impl Statics {
    fn build(px: usize) -> Statics {
        let scale = px as f32 / VB;
        let n = px * px;
        let mut under = vec![[0.0f32; 4]; n];
        let mut clip = vec![0.0f32; n];
        let mut stroke = vec![0.0f32; n];

        let outline = flask_outline();
        let glass = [(59.0f32, 28.0f32), (76.0, 28.0), (85.0, 86.0), (50.0, 86.0)];

        for y in 0..px {
            for x in 0..px {
                let i = y * px + x;
                let p = ((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);

                let mut c = [0.0f32; 4];
                over(&mut c, BG, coverage(rounded_rect_sd(p), scale));

                let mut sd = f32::NEG_INFINITY;
                for k in 0..4 {
                    let a = glass[k];
                    let b = glass[(k + 1) % 4];
                    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
                    let inv_len = 1.0 / (ex * ex + ey * ey).sqrt();
                    let d = ((p.0 - a.0) * ey - (p.1 - a.1) * ex) * inv_len;
                    sd = sd.max(d);
                }
                let cov_clip = coverage(sd, scale);
                clip[i] = cov_clip;

                over(&mut c, DARK, 0.35 * cov_clip);
                under[i] = c;

                let mut d_out = f32::MAX;
                for k in 0..outline.len() {
                    let a = outline[k];
                    let b = outline[(k + 1) % outline.len()];
                    d_out = d_out.min(seg_dist(p, a, b));
                }
                let c_outline = coverage(d_out - 3.5 / 2.0, scale);
                let d_neck = seg_dist(p, (58.0, 31.0), (77.0, 31.0));
                let c_neck = coverage(d_neck - 3.0 / 2.0, scale);
                stroke[i] = 1.0 - (1.0 - c_outline) * (1.0 - c_neck);
            }
        }

        let gx0 = ((36.0 * scale).floor() as usize).min(px);
        let gy0 = ((14.0 * scale).floor() as usize).min(px);
        let gx1 = (((99.0 * scale).ceil() as usize).max(gx0 + 1)).min(px);
        let gy1 = (((100.0 * scale).ceil() as usize).max(gy0 + 1)).min(px);
        let (gw, gh) = (gx1 - gx0, gy1 - gy0);

        let sigma = 4.0 * scale;
        let krad = (3.0 * sigma).ceil() as i32;
        let mut kernel = Vec::with_capacity((2 * krad + 1) as usize);
        let mut sum = 0.0f32;
        for k in -krad..=krad {
            let w = (-(k as f32 * k as f32) / (2.0 * sigma * sigma)).exp();
            kernel.push(w);
            sum += w;
        }
        for w in &mut kernel {
            *w /= sum;
        }

        Statics {
            px,
            scale,
            under,
            clip,
            stroke,
            gx0,
            gy0,
            gw,
            gh,
            kernel,
            krad,
            goo: vec![[0.0; 4]; gw * gh],
            tmp: vec![[0.0; 4]; gw * gh],
            rgba: vec![0; n * 4],
        }
    }

    fn render_goo(&mut self, t: f64) {
        let tri = |period: f64| -> f32 {
            let ph = (t / period).fract();
            (if ph < 0.5 { ph * 2.0 } else { (1.0 - ph) * 2.0 }) as f32
        };
        let k7 = tri(7.0);
        let k9 = tri(9.0);

        struct Blob {
            cx: f32,
            cy: f32,
            rx: f32,
            ry: f32,
        }
        let blobs = [
            Blob { cx: 67.5, cy: 86.0, rx: 17.0, ry: 8.0 },
            Blob {
                cx: 64.0,
                cy: 78.0 + (44.0 - 78.0) * k7,
                rx: 8.0 + (6.0 - 8.0) * k7,
                ry: 8.0 + (6.0 - 8.0) * k7,
            },
            Blob {
                cx: 70.0,
                cy: 44.0 + (76.0 - 44.0) * k9,
                rx: 6.0 + (8.0 - 6.0) * k9,
                ry: 6.0 + (8.0 - 6.0) * k9,
            },
        ];

        for v in self.goo.iter_mut() {
            *v = [0.0; 4];
        }
        let scale = self.scale;
        for gy in 0..self.gh {
            let uy = ((self.gy0 + gy) as f32 + 0.5) / scale;
            for gx in 0..self.gw {
                let ux = ((self.gx0 + gx) as f32 + 0.5) / scale;
                let gi = gy * self.gw + gx;
                for b in &blobs {
                    let dx = (ux - b.cx) / b.rx;
                    let dy = (uy - b.cy) / b.ry;
                    let l = (dx * dx + dy * dy).sqrt();
                    let sd = (l - 1.0) * b.rx.min(b.ry);
                    let cov = coverage(sd, scale);
                    if cov <= 0.0 {
                        continue;
                    }
                    let u = (ux - (b.cx - b.rx)) / (2.0 * b.rx);
                    let v = (uy - (b.cy - b.ry)) / (2.0 * b.ry);
                    let du = u - 0.42;
                    let dv = v - 0.32;
                    let g = ((du * du + dv * dv).sqrt() / 0.8).clamp(0.0, 1.0);
                    let rgb = [
                        GRAD_IN[0] + (GRAD_OUT[0] - GRAD_IN[0]) * g,
                        GRAD_IN[1] + (GRAD_OUT[1] - GRAD_IN[1]) * g,
                        GRAD_IN[2] + (GRAD_OUT[2] - GRAD_IN[2]) * g,
                    ];
                    let d = &mut self.goo[gi];
                    let ia = 1.0 - cov;
                    d[0] = rgb[0] * cov + d[0] * ia;
                    d[1] = rgb[1] * cov + d[1] * ia;
                    d[2] = rgb[2] * cov + d[2] * ia;
                    d[3] = cov + d[3] * ia;
                }
            }
        }

        let (gw, gh) = (self.gw as i32, self.gh as i32);
        let krad = self.krad;
        for y in 0..gh {
            let row = (y * gw) as usize;
            for x in 0..gw {
                let mut acc = [0.0f32; 4];
                let lo = (-x).max(-krad);
                let hi = (gw - 1 - x).min(krad);
                for k in lo..=hi {
                    let w = self.kernel[(k + krad) as usize];
                    let s = self.goo[row + (x + k) as usize];
                    acc[0] += s[0] * w;
                    acc[1] += s[1] * w;
                    acc[2] += s[2] * w;
                    acc[3] += s[3] * w;
                }
                self.tmp[row + x as usize] = acc;
            }
        }
        for y in 0..gh {
            for x in 0..gw {
                let mut acc = [0.0f32; 4];
                let lo = (-y).max(-krad);
                let hi = (gh - 1 - y).min(krad);
                for k in lo..=hi {
                    let w = self.kernel[(k + krad) as usize];
                    let s = self.tmp[((y + k) * gw + x) as usize];
                    acc[0] += s[0] * w;
                    acc[1] += s[1] * w;
                    acc[2] += s[2] * w;
                    acc[3] += s[3] * w;
                }
                self.goo[(y * gw + x) as usize] = acc;
            }
        }
    }

    fn compose(&mut self) {
        let px = self.px;
        for y in 0..px {
            for x in 0..px {
                let i = y * px + x;
                let mut c = self.under[i];

                if x >= self.gx0
                    && x < self.gx0 + self.gw
                    && y >= self.gy0
                    && y < self.gy0 + self.gh
                {
                    let g = self.goo[(y - self.gy0) * self.gw + (x - self.gx0)];
                    let a2 = (g[3] * 20.0 - 9.0).clamp(0.0, 1.0) * self.clip[i];
                    if a2 > 0.0 {
                        let rgb = [g[0] / g[3], g[1] / g[3], g[2] / g[3]];
                        over(&mut c, rgb, a2);
                    }
                }

                over(&mut c, WHITE, self.stroke[i]);

                let o = i * 4;
                self.rgba[o] = (c[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                self.rgba[o + 1] = (c[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                self.rgba[o + 2] = (c[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                self.rgba[o + 3] = (c[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    }
}
