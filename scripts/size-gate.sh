#!/bin/sh
# Size budget check for minfetch (ROADMAP Phase 0, amended by vision review H-11).
# The budget is bendable: it is a reviewed number that grows only for features
# that earn their share. CI always measures and reports the actual size.
# Usage: scripts/size-gate.sh [path-to-binary]  (default: target/release/minfetch)
set -eu

BIN="${1:-target/release/minfetch}"
BUDGET_KIB="${MINFETCH_SIZE_BUDGET_KIB:-1536}" # 1.5 MiB hard ceiling
TARGET_KIB=600                                 # aspiration for the default binary

if [ ! -f "$BIN" ]; then
    echo "size-gate: binary not found: $BIN" >&2
    exit 1
fi

BYTES=$(wc -c < "$BIN" | tr -d '[:space:]')
KB=$((BYTES / 1024))
STATUS="pass"
if [ "$KB" -gt "$BUDGET_KIB" ]; then
    STATUS="FAIL"
elif [ "$KB" -le "$TARGET_KIB" ]; then
    STATUS="on-target"
fi

echo "binary size: ${KB} KiB (${BYTES} bytes) | budget: ${BUDGET_KIB} KiB | on-target: <= ${TARGET_KIB} KiB | status: $STATUS"

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "| binary | size (KiB) | bytes | budget (KiB) | status |"
        echo "|--------|-----------:|------:|-------------:|--------|"
        echo "| $BIN | $KB | $BYTES | $BUDGET_KIB | $STATUS |"
    } >>"$GITHUB_STEP_SUMMARY"
fi

if [ "$STATUS" = "FAIL" ]; then
    echo "size-gate: FAILED — ${KB} KiB exceeds the ${BUDGET_KIB} KiB budget (raise it deliberately in scripts/size-gate.sh if this feature earned its size)" >&2
    exit 1
fi

echo "size-gate: OK"
