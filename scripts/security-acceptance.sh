#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ARTIFACT_ROOT=${PAPERCUTS_ACCEPTANCE_ARTIFACT_ROOT:-"$ROOT/target/security-acceptance"}
RUN_ID=${PAPERCUTS_ACCEPTANCE_RUN_ID:-"suite-$(date -u +%Y%m%dT%H%M%SZ)"}
RUN_DIR="$ARTIFACT_ROOT/$RUN_ID"
LOG="$RUN_DIR/runner.log"
SUMMARY="$RUN_DIR/summary.json"

mkdir -p "$RUN_DIR"
cd "$ROOT"

commit=$(git rev-parse --verify HEAD)
rustc_version=$(rustc --version)
cargo_version=$(cargo --version)

printf '%s\n' \
  'security acceptance preflight' \
  "run_id=$RUN_ID" \
  "commit=$commit" \
  "rustc=$rustc_version" \
  "cargo=$cargo_version" \
  'command=cargo test --all-features --test security_acceptance -- --test-threads=1' \
  'fixtures=synthetic; outputs must identify fixture ids only' | tee "$LOG"

set +e
cargo test --all-features --test security_acceptance -- --test-threads=1 2>&1 \
  | sed -e "s|$ROOT|<repo>|g" -e 's|/home/[^ /]*/\.cache/cargo-target|<cargo-target>|g' \
  | tee -a "$LOG"
status=${PIPESTATUS[0]}
set -e

if [[ $status -eq 0 ]]; then
  result=pass
else
  result=fail
fi

printf '%s\n' \
  "{\"contract\":1,\"run_id\":\"$RUN_ID\",\"result\":\"$result\",\"exit\":$status,\"commit\":\"$commit\",\"command\":\"cargo test --all-features --test security_acceptance -- --test-threads=1\",\"log\":\"runner.log\"}" \
  >"$SUMMARY"

printf '%s\n' "result=$result" "summary=target/security-acceptance/$RUN_ID/summary.json" | tee -a "$LOG"
exit "$status"
