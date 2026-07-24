mod goo_widget;
mod style;

use ed25519_dalek::VerifyingKey;
use quartermaster_license::{fingerprint::fingerprint, verify_any, License};
use serde::Serialize;
use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[cfg(debug_assertions)]
const MIN_DISPLAY_TIME: std::time::Duration = std::time::Duration::from_secs(15);
#[cfg(not(debug_assertions))]
const MIN_DISPLAY_TIME: std::time::Duration = std::time::Duration::from_millis(300);

const PUBLIC_KEYS_HEX: &[&str] = &[
    "eba29494abda910c3670ab0aab126cbca5062130f54c3ad0bcbc9d5aa8d6b9ca",
];

const DOWNLOAD_URL: &str = "https://quartermaster.lauden.dev/license/download";

#[derive(Serialize)]
struct DownloadRequest {
    license_key: String,
    fingerprint: String,
}

fn load_public_keys() -> Vec<VerifyingKey> {
    PUBLIC_KEYS_HEX
        .iter()
        .map(|hex_str| {
            let bytes = hex::decode(hex_str).expect("invalid public key hex in binary");
            let arr: [u8; 32] = bytes.try_into().expect("public key must be 32 bytes");
            VerifyingKey::from_bytes(&arr).expect("invalid public key bytes")
        })
        .collect()
}

enum Status {
    Idle,
    Working(License),
    Success(String),
    Error(String),
}

/// Messages sent from the background download thread back to the UI.
enum WorkerMsg {
    Verified(License),
    Done(String),
    Failed(String),
}

struct LauncherApp {
    pubs: Vec<VerifyingKey>,
    key_input: String,
    status: Status,
    rx: Option<Receiver<WorkerMsg>>,
    goo: goo_widget::GooWidget,
    key_hint_dismissed: bool,
}

impl Default for LauncherApp {
    fn default() -> Self {
        Self {
            pubs: load_public_keys(),
            key_input: String::new(),
            status: Status::Idle,
            rx: None,
            goo: goo_widget::GooWidget::default(),
            key_hint_dismissed: false,
        }
    }
}

impl LauncherApp {
    fn start_download(&mut self) {
        let pubs = self.pubs.clone();
        let key_input = self.key_input.clone();
        let (tx, rx): (Sender<WorkerMsg>, Receiver<WorkerMsg>) = channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            run_download(pubs, key_input, tx);
        });
    }

    fn poll_worker(&mut self) {
        let Some(rx) = &self.rx else { return };
        // try_recv is non-blocking — returns immediately if no message
        // has arrived yet, which is what lets the UI keep repainting
        // (and animating) while the background thread is still working.
        match rx.try_recv() {
            Ok(WorkerMsg::Verified(license)) => {
                self.status = Status::Working(license);
            }
            Ok(WorkerMsg::Done(filename)) => {
                self.status = Status::Success(filename);
                self.rx = None;
            }
            Ok(WorkerMsg::Failed(msg)) => {
                self.status = Status::Error(msg);
                self.rx = None;
            }
            Err(_) => {} // nothing new yet, keep waiting
        }
    }
}

fn run_download(pubs: Vec<VerifyingKey>, key_input: String, tx: Sender<WorkerMsg>) {
    let start = std::time::Instant::now();

    let license = match verify_any(&pubs, &key_input) {
        Ok(l) => l,
        Err(e) => {
            let _ = tx.send(WorkerMsg::Failed(format!("License failed to verify: {:?}", e)));
            return;
        }
    };

    let _ = tx.send(WorkerMsg::Verified(license.clone()));

    let fp = match fingerprint(&license.product) {
        Ok(f) => f,
        Err(_) => {
            let _ = tx.send(WorkerMsg::Failed("Could not read this machine's identifier.".into()));
            return;
        }
    };

    let body = DownloadRequest {
        license_key: key_input,
        fingerprint: fp,
    };

    let response = match ureq::post(DOWNLOAD_URL).send_json(&body) {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(WorkerMsg::Failed(format!("Download request failed: {}", e)));
            return;
        }
    };

    let filename = format!("{}.zip", license.product);
    let file = match fs::File::create(&filename) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(WorkerMsg::Failed(format!("Could not save file: {}", e)));
            return;
        }
    };

    let mut file = file;
    let mut reader = response.into_reader();
    if let Err(e) = std::io::copy(&mut reader, &mut file) {
        let _ = tx.send(WorkerMsg::Failed(format!("Could not write file: {}", e)));
        return;
    }

    let elapsed = start.elapsed();
    if elapsed < MIN_DISPLAY_TIME {
        thread::sleep(MIN_DISPLAY_TIME - elapsed);
    }

    let _ = tx.send(WorkerMsg::Done(filename));
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker();

        // Request a repaint even with no user input, so the animation
        // keeps moving and we keep checking the channel while waiting.
        if self.rx.is_some() {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Single title, living only in the OS titlebar (set in main()
            // via with_title). No in-window heading — that was the
            // second/third "title" showing up before.
            ui.add_space(20.0);

            let working = self.rx.is_some();

            // Left-aligned form block, indented from the panel edge —
            // not centered. vertical_centered was pulling the text edit
            // to the middle of the window; a plain indented vertical
            // layout reads as a normal left-to-right form instead.
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                ui.vertical(|ui| {
                    // Once a download starts, the key input disappears
                    // entirely (not just disabled) so the goo animation
                    // is the only thing in view — matches a real
                    // "working" state instead of a greyed-out form.
                    if !working {
                        let input_width = 280.0;
                        ui.horizontal(|ui| {
                            // Built here, not before the closure — building
                            // it earlier held a mutable borrow of
                            // self.key_input open for the closure's whole
                            // body, which then collided with the second
                            // &mut self.key_input the Paste button needs
                            // below (E0499).
                            let edit = egui::TextEdit::singleline(&mut self.key_input).hint_text(
                                if self.key_hint_dismissed {
                                    ""
                                } else {
                                    "Paste your license key"
                                },
                            );
                            let response = ui.add_sized([input_width, 32.0], edit);
                            // Hint clears as soon as the field is clicked
                            // into, not only once typing starts (egui's
                            // actual default) — matches normal form-field
                            // behavior.
                            if response.gained_focus() {
                                self.key_hint_dismissed = true;
                            }
                            // egui's TextEdit has no right-click context
                            // menu (Ctrl+V still works) — this button is
                            // the explicit, discoverable paste path for
                            // anyone reaching for right-click out of habit.
                            if ui.add_sized([70.0, 32.0], egui::Button::new("Paste")).clicked() {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    if let Ok(text) = clipboard.get_text() {
                                        self.key_input = text.trim().to_string();
                                        self.key_hint_dismissed = true;
                                    }
                                }
                            }
                        });
                        ui.add_space(14.0);

                        // "Lights up" once there's real (non-whitespace)
                        // text — disabled otherwise, so an empty click
                        // can't fire start_download with nothing to verify.
                        let has_key = !self.key_input.trim().is_empty();
                        ui.add_enabled_ui(has_key, |ui| {
                            if ui
                                .add_sized([160.0, 36.0], egui::Button::new("Download"))
                                .clicked()
                            {
                                self.start_download();
                            }
                        });
                        ui.add_space(16.0);
                    }

                    match &self.status {
                        Status::Idle => {}
                        Status::Working(license) => {
                            // Animated trailing dots: cycles 0..3 dots,
                            // ~2 per second, driven off egui's frame
                            // clock rather than a stored timer — same
                            // pattern as the goo widget's own animation.
                            let t = ui.input(|i| i.time);
                            let dot_count = ((t * 2.0) as usize) % 4;
                            let dots = ".".repeat(dot_count);
                            ui.label(format!("Downloading {}{}", license.product, dots));
                            ui.add_space(8.0);
                            self.goo.show(ui, 96.0);
                        }
                        Status::Success(filename) => {
                            ui.colored_label(
                                egui::Color32::GREEN,
                                format!("Downloaded {}", filename),
                            );
                        }
                        Status::Error(msg) => {
                            ui.colored_label(egui::Color32::RED, msg);
                        }
                    }
                });
            });
        });
    }
}

fn load_icon() -> egui::IconData {
    // Icon is embedded into the binary at compile time via include_bytes!,
    // so the launcher has no runtime dependency on a loose file sitting
    // next to the executable — it can't go missing after packaging.
    let bytes = include_bytes!("../assets/icon_256.png");
    let image = image::load_from_memory(bytes)
        .expect("embedded icon PNG is malformed")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 220.0])
            .with_title("Launcher | shop.lauden.dev")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Launcher | shop.lauden.dev",
        options,
        Box::new(|cc| {
            style::apply(&cc.egui_ctx);
            Ok(Box::new(LauncherApp::default()) as Box<dyn eframe::App>)
        }),
    )
}
