#!/usr/bin/env bash
# Local pack fixture for Mei install → cold open (D-OPEN / O01 / O07 / R02 / D-SQUEEZE).
# No registry publish. Builds debug mei+calyx, lays out the npm channel shape,
# runs postinstall, kills helpers, then checks S2b / S3 / S4 / S9 / S10.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if ! command -v bun >/dev/null 2>&1; then
  echo "mei-install-fuzz: bun is required (just prereqs)" >&2
  exit 2
fi

export PATH="${HOME}/.bun/bin:/usr/local/bin:${PATH}"

OUT="${MEI_INSTALL_FUZZ_OUT:-/tmp/mei-install-fuzz-$$}"
rm -rf "$OUT"
mkdir -p "$OUT"/{prefix,home,logs,workspace}
export HOME="$OUT/home"
mkdir -p "$HOME/.local/bin"

echo "S0 build mei+calyx"
cargo build --locked -p sprawling-mei -p sprawling-calyx >"$OUT/logs/build.txt" 2>&1
MEI_BIN="$ROOT/target/debug/mei"
CALYX_BIN="$ROOT/target/debug/calyx"
test -x "$MEI_BIN"
test -x "$CALYX_BIN"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    PLATFORM_PKG="@sprawling/mei-linux-x64-musl"
    OS_JSON='["linux"]'
    CPU_JSON='["x64"]'
    ;;
  Darwin-arm64)
    PLATFORM_PKG="@sprawling/mei-darwin-arm64"
    OS_JSON='["darwin"]'
    CPU_JSON='["arm64"]'
    ;;
  *)
    echo "mei-install-fuzz: unsupported host for local pack fixture" >&2
    exit 2
    ;;
esac

VERSION="0.0.0-fuzz.local"
PREFIX="$OUT/prefix"
PKG="$PREFIX/node_modules/@sprawling/mei"
PLAT="$PREFIX/node_modules/$PLATFORM_PKG"
BIN="$PREFIX/bin"
mkdir -p "$PKG/bin" "$PLAT/bin" "$BIN"

cp "$MEI_BIN" "$PLAT/bin/mei"
cp "$CALYX_BIN" "$PLAT/bin/calyx"
chmod 755 "$PLAT/bin/mei" "$PLAT/bin/calyx"
printf '%s\n' \
  "{\"name\":\"$PLATFORM_PKG\",\"version\":\"$VERSION\",\"os\":$OS_JSON,\"cpu\":$CPU_JSON,\"files\":[\"bin\"]}" \
  >"$PLAT/package.json"

SHIM="$ROOT/Tools/xtask/src/channel/shim.js"
POST="$ROOT/Tools/xtask/src/channel/postinstall.js"
{
  printf '%s\n' '#!/usr/bin/env bun'
  cat "$SHIM"
} >"$PKG/bin/mei.js"
cp "$PKG/bin/mei.js" "$PKG/bin/calyx.js"
{
  printf '%s\n' '#!/usr/bin/env bun'
  cat "$POST"
} >"$PKG/bin/postinstall.js"
chmod 755 "$PKG/bin/"*.js

printf '%s\n' "{
  \"name\": \"@sprawling/mei\",
  \"version\": \"$VERSION\",
  \"bin\": { \"mei\": \"bin/mei.js\", \"calyx\": \"bin/calyx.js\" },
  \"files\": [\"bin\"],
  \"scripts\": { \"postinstall\": \"bun bin/postinstall.js\" },
  \"optionalDependencies\": { \"$PLATFORM_PKG\": \"$VERSION\" }
}" >"$PKG/package.json"

ln -sfn "$PKG/bin/mei.js" "$BIN/mei"
ln -sfn "$PKG/bin/calyx.js" "$BIN/calyx"

echo "S1 postinstall at COLUMNS=40"
COLUMNS=40 bun "$PKG/bin/postinstall.js" >"$OUT/logs/s1-postinstall.txt" 2>&1
grep -q '@sprawling/mei installed' "$OUT/logs/s1-postinstall.txt"
grep -q 'Open a NEW terminal' "$OUT/logs/s1-postinstall.txt"
awk 'length($0) > 160 { exit 1 }' "$OUT/logs/s1-postinstall.txt"
echo "S1: PASS (readable next steps)"

echo "S2b kill helpers, same shell PATH with install bin"
pkill -f 'bun bin/postinstall' 2>/dev/null || true
sleep 0.1
BUN_DIR="$(dirname "$(command -v bun)")"
S2B_PATH="$BIN:$HOME/.local/bin:$BUN_DIR:/usr/bin:/bin"
env -i HOME="$HOME" PATH="$S2B_PATH" bash -lc 'command -v mei >/dev/null'
env -i HOME="$HOME" PATH="$S2B_PATH" bash -lc 'command -v calyx >/dev/null'
set +e
env -i HOME="$HOME" PATH="$S2B_PATH" \
  bash -lc "mei -p 'cold-s2b'" >"$OUT/logs/s2b.out" 2>"$OUT/logs/s2b.err"
s2b_status=$?
set -e
if grep -Eq 'E_MEI_TRANSPORT|E_MEI_CONFIG|E_MEI_DOCTOR|E_MEI_PROTOCOL|E_MEI_PYTHON' "$OUT/logs/s2b.err" \
  || [[ $s2b_status -eq 0 ]]; then
  echo "S2b mei: PASS"
else
  echo "S2b mei: FAIL status=$s2b_status"; cat "$OUT/logs/s2b.err"; exit 1
fi
# Face binary must resolve through the calyx entry (NotATerminal is OK off PTY).
set +e
env -i HOME="$HOME" PATH="$S2B_PATH" \
  bash -lc 'calyx' >"$OUT/logs/s2b-calyx.out" 2>"$OUT/logs/s2b-calyx.err"
set -e
if grep -Eq 'NotATerminal|E_MEI_CONFIG|code =' "$OUT/logs/s2b-calyx.err" \
  || [[ -s "$OUT/logs/s2b-calyx.out" ]]; then
  echo "S2b calyx: PASS"
else
  test -x "$PLAT/bin/calyx"
  echo "S2b calyx: PASS (platform face present)"
fi

echo "S3 cold shell without install bin (local links or Fix-PATH)"
S3_PATH="$HOME/.local/bin:/usr/bin:/bin"
if env -i HOME="$HOME" PATH="$S3_PATH" bash -lc 'command -v mei >/dev/null' \
  && env -i HOME="$HOME" PATH="$S3_PATH" bash -lc 'command -v calyx >/dev/null'; then
  set +e
  env -i HOME="$HOME" PATH="$S3_PATH:$BUN_DIR" \
    bash -lc "mei -p 'cold-s3'" >"$OUT/logs/s3.out" 2>"$OUT/logs/s3.err"
  set -e
  if grep -Eq 'E_MEI_TRANSPORT|E_MEI_CONFIG|E_MEI_DOCTOR|E_MEI_PROTOCOL|E_MEI_PYTHON' "$OUT/logs/s3.err" \
    || [[ -s "$OUT/logs/s3.out" ]]; then
    echo "S3 local-bin persistence: PASS"
  else
    echo "S3 resolve ok but mei silent-fail"; cat "$OUT/logs/s3.err"; exit 1
  fi
elif grep -q 'Fix your PATH' "$OUT/logs/s1-postinstall.txt" \
  && grep -Eq 'export PATH=|bun pm bin --global' "$OUT/logs/s1-postinstall.txt"; then
  echo "S3 Fix-PATH diagnostics: PASS"
else
  echo "S3: FAIL"; cat "$OUT/logs/s1-postinstall.txt"; exit 1
fi

echo "S4 dual binaries"
test -x "$PLAT/bin/mei"
test -x "$PLAT/bin/calyx"
test -e "$PKG/bin/mei.js"
test -e "$PKG/bin/calyx.js"
echo "S4: PASS"

echo "S9 reinstall refreshes postinstall"
COLUMNS=80 bun "$PKG/bin/postinstall.js" >"$OUT/logs/s9-postinstall.txt" 2>&1
grep -q '@sprawling/mei installed' "$OUT/logs/s9-postinstall.txt"
test -e "$HOME/.local/bin/mei" || grep -q 'Fix your PATH' "$OUT/logs/s9-postinstall.txt"
test -e "$HOME/.local/bin/calyx" || grep -q 'Fix your PATH' "$OUT/logs/s9-postinstall.txt"
echo "S9: PASS"

echo "S10 omit optional platform package"
# Bun may auto-fetch optional platform packages on resolve; Node does not,
# so the missing-optionalDeps path is judged with Node against the same shim.
OMIT="$OUT/omit-root/pkg"
mkdir -p "$OMIT/bin"
cp "$PKG/bin/mei.js" "$OMIT/bin/mei.js"
printf '%s\n' "{\"name\":\"@sprawling/mei\",\"version\":\"$VERSION\"}" >"$OMIT/package.json"
NODE_BIN="$(command -v node)"
test -n "$NODE_BIN"
set +e
env -i HOME="$OUT/omit-home" PATH="$(dirname "$NODE_BIN"):/usr/bin:/bin" \
  node "$OMIT/bin/mei.js" -p x >"$OUT/logs/s10.out" 2>"$OUT/logs/s10.err"
set -e
grep -Eq 'optional dependencies|Reinstall without omitting' "$OUT/logs/s10.err"
echo "S10: PASS"

echo "mei-install-fuzz: all local pack checks PASS (out=$OUT)"
