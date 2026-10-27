#!/bin/sh
# Prova end-to-end su due veri volumi exFAT (immagini disco), solo macOS.
# Uso: scripts/exfat-e2e.sh
set -eu

cd "$(dirname "$0")/.."
ROOT="$PWD/dev-disks/exfat"
A="$ROOT/mnt-a"
B="$ROOT/mnt-b"
export HDDRAID_WORK="$ROOT/work"

cleanup() {
    hdiutil detach "$A" -quiet 2>/dev/null || true
    hdiutil detach "$B" -quiet 2>/dev/null || true
}
trap cleanup EXIT

cleanup
rm -rf "$ROOT"
mkdir -p "$ROOT"
for d in a b; do
    hdiutil create -quiet -size 64m -fs ExFAT -volname "HDDTEST-$d" -o "$ROOT/$d.dmg"
    hdiutil attach -quiet -nobrowse -mountpoint "$ROOT/mnt-$d" "$ROOT/$d.dmg"
done
mount | grep -q "$A (exfat" || { echo "Il disco A non è exFAT"; exit 1; }

cargo build -q -p hddraid-devtools -p hddraid-cli
DEV="target/debug/dev-disks"
CLI="target/debug/hddraid"

"$DEV" setup --a "$A" --b "$B"
"$CLI" init "$A" "$B"
"$CLI" sync "$A" "$B" --yes
"$DEV" break all --a "$A" --b "$B"
# Conflitto: tienile entrambe; cancellazione: rimetti.
printf 'e\nr\ns\n' | "$CLI" sync "$A" "$B" >/dev/null
"$CLI" check "$A" "$B" --repair 2>/dev/null

echo "--- Secondo allineamento (deve essere vuoto)"
OUT=$("$CLI" sync "$A" "$B" --yes)
echo "$OUT"
[ "$OUT" = "I dischi sono già uguali." ] || { echo "ERRORE: i dischi non sono allineati"; exit 1; }

echo "--- Differenze rimaste (solo file di sistema ignorati)"
DIFF=$(diff -rq -x .hddraid -x .fseventsd -x .DS_Store -x Thumbs.db -x '$RECYCLE.BIN' -x '._*' "$A" "$B" || true)
[ -z "$DIFF" ] || { echo "$DIFF"; echo "ERRORE: contenuti diversi"; exit 1; }
ls "$B/Documenti"
echo "OK: dischi exFAT allineati"
