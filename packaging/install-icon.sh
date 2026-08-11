#!/bin/bash
# One-time install: registers the launcher with KDE's app database so
# Wayland/KWin can look up its titlebar icon by identity instead of
# falling back to a generic "W" placeholder. Re-run after changing the
# icon artwork or moving the binary.
set -euo pipefail

ICON_NAME="lauden-launcher"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"  # launcher/

echo "Installing icons into ~/.local/share/icons/hicolor/..."
for size in 16 32 64 128 256; do
    dest="$HOME/.local/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$dest"
    src="$SRC_DIR/packaging/icon_${size}.png"
    if [ -f "$src" ]; then
        cp "$src" "$dest/${ICON_NAME}.png"
    else
        echo "  (skipping ${size}x${size} — icon_${size}.png not found in packaging/)"
    fi
done

echo "Installing .desktop file..."
mkdir -p "$HOME/.local/share/applications"
cp "$SRC_DIR/packaging/lauden-launcher.desktop" \
   "$HOME/.local/share/applications/lauden-launcher.desktop"

echo "Refreshing caches..."
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true

echo "Done. Log out/in (or restart kwin_wayland: kwin_wayland --replace &) for KDE to pick it up."
