#!/usr/bin/env bash
# Runs every playbook headless, several at once, and fails if any does.
# Usage, from projects/: zawnabout-playbooks/run-all.sh OUTDIR [JOBS]
set -u
cd "$(dirname "$0")/.."
out="${1:?usage: run-all.sh OUTDIR [JOBS]}"
jobs="${2:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}"
exe=target/release/zawnabout
[ -f "$exe.exe" ] && exe="$exe.exe"
mkdir -p "$out"
rm -f "$out"/*.failed
export exe out
ls zawnabout-playbooks/*.txt | xargs -P "$jobs" -I{} bash -c '
  n=$(basename "{}" .txt)
  start=$(date +%s)
  if "$exe" --headless --script "{}" --out "$out/$n" > "$out/$n.log" 2>&1; then
    echo "ok   $n ($(( $(date +%s) - start ))s)"
  else
    echo "FAIL $n ($(( $(date +%s) - start ))s)"; tail -5 "$out/$n.log"; touch "$out/$n.failed"
  fi
'
failed=$(ls "$out"/*.failed 2>/dev/null | wc -l | tr -d " ")
echo "playbooks done, $failed failed"
[ "$failed" = 0 ]
