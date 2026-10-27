#!/bin/sh
# Linux, solo sviluppo: registra l'icona dell'app per `pnpm tauri dev`.
#
# Su Wayland il desktop non usa l'icona impostata dall'app: cerca un file
# .desktop con lo stesso nome dell'app_id della finestra, che per Tauri è il
# nome del binario. Il pacchetto installato porta già il suo (HddRaid.desktop),
# il binario di sviluppo `hddraid-app` no, e KDE mostra l'icona generica.
#
# Rilanciare dopo aver cambiato l'icona. Per togliere tutto: `--remove`.
set -eu

NAME=hddraid-app
ICONS="$(cd "$(dirname "$0")/../src-tauri/icons" && pwd)"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
DESKTOP="$DATA/applications/$NAME.desktop"

for size in 32x32 128x128 256x256; do
  dir="$DATA/icons/hicolor/$size/apps"
  if [ "${1:-}" = "--remove" ]; then
    rm -f "$dir/$NAME.png"
    continue
  fi
  case $size in
    256x256) src="$ICONS/128x128@2x.png" ;;
    *) src="$ICONS/$size.png" ;;
  esac
  mkdir -p "$dir"
  cp "$src" "$dir/$NAME.png"
done

if [ "${1:-}" = "--remove" ]; then
  rm -f "$DESKTOP"
else
  mkdir -p "$(dirname "$DESKTOP")"
  cat >"$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=HddRaid (dev)
Exec=$NAME
Icon=$NAME
StartupWMClass=$NAME
Terminal=false
NoDisplay=true
EOF
fi

# Fa rileggere icone e .desktop al desktop, se gli strumenti ci sono.
gtk-update-icon-cache -q -t "$DATA/icons/hicolor" 2>/dev/null || true
update-desktop-database -q "$DATA/applications" 2>/dev/null || true
kbuildsycoca6 --noincremental >/dev/null 2>&1 || kbuildsycoca5 >/dev/null 2>&1 || true
