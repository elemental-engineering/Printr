#!/usr/bin/env bash
# Régénère les exemples de blocs (docs/exemples/*.png) dans l'émulateur.
# Usage : docs/exemples/generer.sh [bloc…]   (défaut : tous les .json du dossier)
# Prérequis : uv et l'émulateur (uvx emupos).
set -euo pipefail
cd "$(dirname "$0")/../.."

PORT=9100
LOG=$(mktemp)
port_open() { (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; }
# L'émulateur tourne dans son propre groupe de processus, pour l'arrêter en entier (uvx et Python).
EMU=
trap '[ -n "$EMU" ] && kill -- -"$EMU" 2>/dev/null; rm -f "$LOG"' EXIT

cargo build --release -q
BIN=target/release/printr
export PRINTR_GLITCH=0

if port_open; then echo "le port $PORT est déjà utilisé" >&2; exit 1; fi
(cd emulator && exec setsid uvx emupos run >"$LOG" 2>&1) &
EMU=$!
until port_open; do sleep 0.3; done
mkdir -p emulator/receipts

names=("$@")
[ ${#names[@]} -eq 0 ] && names=($(ls docs/exemples/*.json | xargs -n1 basename | sed 's/\.json$//'))
for name in "${names[@]}"; do
  before=$(find emulator/receipts -name '*.png' | wc -l)
  $BIN -q --tcp "127.0.0.1:$PORT" print "docs/exemples/$name.json"
  until [ "$(find emulator/receipts -name '*.png' | wc -l)" -gt "$before" ]; do sleep 0.3; done
  cp "$(ls -t emulator/receipts/*.png | head -1)" "docs/exemples/$name.png"
  chmod 644 "docs/exemples/$name.png"
  echo "docs/exemples/$name.png"
done
