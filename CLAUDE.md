# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`lauden-dev-launcher` is a small egui/eframe desktop GUI (Linux/KDE-Wayland target) that lets a
customer paste a Lauden.dev product license key, verifies it locally, and downloads the
purchased product as a zip from `quartermaster.lauden.dev`. It's intentionally product-agnostic —
the product name comes from the verified license, not from a build-time config.

## Build / run

```bash
cargo build              # debug build
cargo build --release    # release build (strip + lto enabled, see Cargo.toml)
cargo run                # run the launcher locally
cargo check               # fast type-check without codegen
```

### Cross-platform builds

`ureq` uses `rustls` (not OpenSSL) and `arboard` already carries per-platform backends
(`clipboard-win` / `objc2`), so there's no system-TLS dependency to fight when targeting other
OSes.

- **Linux X11**: no separate build — eframe 0.28's default features enable both the `x11` and
  `wayland` winit backends, so the normal `cargo build --release` binary runs under either
  session. Just make sure X11 dev headers are installed at build time (e.g. Fedora/Nobara:
  `libX11-devel libXcursor-devel libXrandr-devel libXi-devel libxkbcommon-devel`; Debian/Ubuntu:
  `libx11-dev libxcursor-dev libxrandr-dev libxi-dev libxkbcommon-dev`).
- **Windows** (cross-compiled from Linux):
  ```bash
  rustup target add x86_64-pc-windows-gnu
  sudo dnf install mingw64-gcc mingw64-gcc-c++ mingw64-winpthreads-static   # Debian/Ubuntu: gcc-mingw-w64-x86-64
  cargo build --release --target x86_64-pc-windows-gnu
  ```
  Output: `target/x86_64-pc-windows-gnu/release/lauden-dev-launcher.exe`. `main.rs` sets
  `#![windows_subsystem = "windows"]` at the top of the file — without it, an eframe binary built
  with the default (console) subsystem opens a terminal window behind the GUI on Windows. That
  attribute is a no-op on non-Windows targets, so it's left unconditional rather than
  `#[cfg(windows)]`-gated.
- **macOS**: cross-compiling to macOS from Linux isn't practical (Apple's SDK license doesn't
  permit redistributing it for cross-toolchains). Build natively on a Mac instead:
  ```bash
  # both this repo and the sibling ../quartermaster-license repo must be present,
  # in the same relative layout, on the Mac (path dependency, see below)
  xcode-select --install        # once, if not already installed
  curl https://sh.rustup.rs -sSf | sh   # once, if rustup isn't already installed
  cargo build --release          # aarch64-apple-darwin is the default target on Apple Silicon
  ```
  Output: `target/release/lauden-dev-launcher`. To also produce an Intel binary from an Apple
  Silicon Mac, Xcode's cross-linking makes `rustup target add x86_64-apple-darwin && cargo build
  --release --target x86_64-apple-darwin` work without Rosetta; combine the two into one universal
  binary with `lipo -create -output lauden-dev-launcher-universal target/aarch64-apple-darwin/release/lauden-dev-launcher target/x86_64-apple-darwin/release/lauden-dev-launcher` if desired.

There are no automated tests in this repo currently. There is no separate lint step configured
beyond `cargo check`/`cargo clippy` if you choose to run it manually.

Debug builds artificially pad the download flow to a minimum of 15s (`MIN_DISPLAY_TIME` in
`main.rs`) so the goo animation is easy to see while developing; release builds use 300ms. Keep
this in mind if a debug run "hangs" after a fast download — it's intentional.

## Local workspace layout (not a Cargo workspace)

This crate depends on a **sibling repository**, not a published crate:

```
Cargo.toml: quartermaster-license = { path = "../quartermaster-license" }
```

`../quartermaster-license` is a separate git repo (own `.git`, own `Cargo.lock`) that lives next
to this one on disk. It provides:
- `License`, `verify`, `verify_any`, `LicenseError` (`license.rs`) — ed25519-signed license
  parsing/verification.
- `fingerprint::fingerprint` — per-machine identifier used to bind a download to this machine.
- `storage::load_or_prompt_license` — not currently used by this launcher's UI flow.

When working on license verification behavior, the relevant logic often lives in that sibling
repo, not here. This launcher only *consumes* its public API (`verify_any`, `License`,
`fingerprint`).

## Architecture

Three files, each with a distinct responsibility:

- **`src/main.rs`** — app state machine and I/O. `LauncherApp` has one `Status` enum
  (`Idle` / `Working(License)` / `Success(filename)` / `Error(msg)`) driving what the UI shows.
  Verification + download run on a spawned `thread::spawn`, communicating back to the UI thread
  over an `mpsc::channel` (`WorkerMsg::Verified` / `Done` / `Failed`), polled non-blockingly each
  frame via `poll_worker()`. This keeps the goo animation and dot-cycling responsive while the
  network request is in flight — never do license verification or `ureq` calls directly on the
  UI thread.
  - `PUBLIC_KEYS_HEX` holds the ed25519 public key(s) baked into the binary that
    `verify_any` checks the pasted key against; `DOWNLOAD_URL` is the fixed backend endpoint.
  - `FORM_WIDTH` / `MARGIN` / `WINDOW_HEIGHT` are the single source of truth for window sizing —
    the window is sized in `main()` from the same constants the form layout uses in `update()`,
    so they can't drift apart. If you change form contents' height, update the `WINDOW_HEIGHT`
    comment's budget breakdown alongside it.

- **`src/style.rs`** — theme applied once at startup (`style::apply`, called from the
  `run_native` setup closure). It's a hand-translated port of a companion web project's
  `style.css` design tokens (brand blue / card bg / border / radius / muted ink), light and dark
  variants selected from the OS-reported `prefers-color-scheme` equivalent
  (`ctx.style().visuals.dark_mode`). If the web styling changes, this is the file to keep in sync
  — the token names in the comments (`--brand-blue`, `--card-bg`, etc.) map directly to CSS
  custom properties in that other project's stylesheet.

- **`src/goo_widget.rs`** — a self-contained, embeddable CPU-rendered "goo in a flask" animation
  shown during `Status::Working`. It's a software rendering pipeline (signed-distance shapes →
  Gaussian blur → alpha threshold recomposite), ported from an SVG/filter-based standalone demo
  (`flask_goo`) into a reusable `GooWidget` with a single `show(ui, size_pts)` entry point. It
  renders into a CPU pixel buffer and uploads it as an egui texture every frame — there's no
  persistent animation state beyond the current time (`ctx.input(|i| i.time)`), so all motion is
  computed fresh each call from `t`. Treat the block under `// --- everything below is unchanged
  from flask_goo's implementation ---` as ported code — prefer keeping it in sync with the
  original demo's math rather than diverging.

## Packaging (Linux desktop integration)

`packaging/` holds Linux desktop-entry integration, not used by `cargo build`/`cargo run`:
- `lauden-launcher.desktop` — references the release binary by absolute path and sets
  `StartupWMClass=lauden-launcher`, which must match `.with_app_id("lauden-launcher")` in
  `main()` for KWin/Wayland to associate the running window with this desktop entry's icon.
- `install-icon.sh` — one-time/idempotent installer that copies `icon_*.png` into
  `~/.local/share/icons/hicolor/...` and the `.desktop` file into
  `~/.local/share/applications/`, then refreshes desktop/icon caches. Re-run it after changing
  icon artwork or moving the release binary.
- The app icon shown in-window (`load_icon()` in `main.rs`) is embedded via `include_bytes!` from
  `assets/icon_256.png` at compile time, independent of the packaging icons above (which are for
  the OS window manager / app launcher, not the eframe window itself).

## Known constraints worth knowing before touching clipboard/paste code

A dedicated in-app "Paste" button was attempted and reverted (see comment in `main.rs` around the
key input field): `arboard`'s clipboard read fails under this environment's Wayland session (a
MIME negotiation issue not fixed by upgrading to arboard 3.6.1), and installing `wl-clipboard` as
a workaround wasn't possible due to a broken repo signature. Native `Ctrl+V` into the `TextEdit`
already works, so this isn't blocking — but don't reintroduce an arboard-based paste button
without first confirming the underlying clipboard issue is actually fixed upstream.
