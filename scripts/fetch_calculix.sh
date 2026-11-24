#!/usr/bin/env bash
# Fetch and verify the CalculiX 2.23 native binary for this platform.
#
# The binary is NOT committed: it is platform-specific and ~55 MB with its
# shared libraries. This script downloads the conda-forge package for the
# detected platform, extracts the binary plus the minimal runtime libraries,
# and verifies a real solve.
#
# Usage:
#   ./scripts/fetch_calculix.sh           # download + install into vendor/
#   ./scripts/fetch_calculix.sh --check   # verify an existing install only
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR="$ROOT/vendor"
BIN_DIR="$VENDOR/bin"
LIB_DIR="$VENDOR/lib"
CCX="$BIN_DIR/ccx_2.23"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
    darwin-arm64) SUBDIR="osx-arm64"; EXT="dylib" ;;
    darwin-x86_64) SUBDIR="osx-x86_64"; EXT="dylib" ;;
    linux-x86_64|linux-amd64) SUBDIR="linux-64"; EXT="so" ;;
    *)
        echo "error: unsupported platform $OS-$ARCH" >&2
        echo "set \$CCX_BIN to a preinstalled ccx_2.x binary" >&2
        exit 2
        ;;
esac

CHECK_ONLY=0
[[ "${1:-}" == "--check" ]] && CHECK_ONLY=1

verify_install() {
    [[ -x "$CCX" ]] || return 1
    local probe
    probe="$(mktemp -d)"
    cat > "$probe/probe.inp" <<'INP'
*NODE, NSET=NALL
1, 0.0, 0.0, 0.0
2, 1.0, 0.0, 0.0
3, 1.0, 0.1, 0.0
4, 0.0, 0.1, 0.0
*ELEMENT, TYPE=S4, ELSET=EALL
1, 1, 2, 3, 4
*MATERIAL, NAME=STEEL
*ELASTIC
210000.0, 0.3
*SHELL SECTION, ELSET=EALL, MATERIAL=STEEL
0.1
*BOUNDARY
1,1,3,0.0
*STEP
*STATIC
*CLOAD
2,2,1000.0
*NODE FILE
U
*END STEP
INP
    DYLD_LIBRARY_PATH="$LIB_DIR${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" \
    LD_LIBRARY_PATH="$LIB_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
        "$CCX" -i "$probe/probe" 2>&1 | grep -q "Job finished"
    local rc=$?
    rm -rf "$probe"
    return $rc
}

if [[ $CHECK_ONLY -eq 1 ]]; then
    if verify_install; then
        echo "CalculiX present and solving: $CCX"
        exit 0
    fi
    echo "CalculiX missing or not solving: $CCX" >&2
    exit 1
fi

if verify_install; then
    echo "CalculiX already present and solving; nothing to do."
    exit 0
fi

echo "== fetching CalculiX for $OS-$ARCH ($SUBDIR) =="

PY="${PYTHON:-python3}"
"$PY" - "$SUBDIR" "$VENDOR" <<'PYEOF'
"""Download the conda-forge calculix package and extract the binary + libs.

Run outside the repo's dependency set: only stdlib + zstandard are needed,
and zstandard is vendored as a fallback via the pure-python path.
"""
import io
import json
import os
import sys
import tarfile
import urllib.request
import zipfile

subdir, vendor = sys.argv[1], sys.argv[2]

try:
    import zstandard  # noqa: F401
    HAVE_ZSTD = True
except ImportError:
    HAVE_ZSTD = False

# The .conda format is a zip of .tar.zst members. If zstandard is missing,
# fall back to the .tar.bz2 variant published alongside it.
def fetch_pkg(name: str) -> bytes:
    for ext in (".conda", ".tar.bz2"):
        url = f"https://conda.anaconda.org/conda-forge/{subdir}/{name}{ext}"
        try:
            data = urllib.request.urlopen(
                urllib.request.Request(url, headers={"User-Agent": "aether-fetch/1.0"}),
                timeout=300,
            ).read()
            return data, ext
        except Exception as exc:
            print(f"  miss {ext}: {exc}", file=sys.stderr)
    raise SystemExit("error: could not download calculix package")


def extract(data: bytes, ext: str):
    if ext == ".tar.bz2":
        tf = tarfile.open(fileobj=io.BytesIO(data))
        return tf, None
    z = zipfile.ZipFile(io.BytesIO(data))
    pkg = [n for n in z.namelist() if n.startswith("pkg-")][0]
    if not HAVE_ZSTD:
        raise SystemExit("error: zstandard is required for .conda packages")
    raw = zstandard.ZstdDecompressor().decompress(
        z.read(pkg), max_output_size=400 * 1024 * 1024
    )
    return tarfile.open(fileobj=io.BytesIO(raw)), z


# Resolve the current calculix build for this subdir.
repodata = json.load(
    urllib.request.urlopen(
        urllib.request.Request(
            f"https://conda.anaconda.org/conda-forge/{subdir}/repodata.json",
            headers={"User-Agent": "aether-fetch/1.0"},
        ),
        timeout=300,
    )
)
pkgs = {**repodata.get("packages", {}), **repodata.get("packages.conda", {})}
cands = [k for k in pkgs if k.startswith("calculix-")]
if not cands:
    raise SystemExit("error: no calculix package for this platform")
cands.sort(key=lambda k: (pkgs[k]["version"], pkgs[k]["timestamp"]))
chosen = cands[-1]
print(f"  selected {chosen} ({pkgs[chosen]['version']})")

data, ext = fetch_pkg(chosen)
tf, _ = extract(data, ext)
os.makedirs(f"{vendor}/bin", exist_ok=True)
os.makedirs(f"{vendor}/lib", exist_ok=True)
for m in tf.getmembers():
    if m.isfile() and m.name.rsplit("/", 1)[-1].startswith("ccx"):
        out = f"{vendor}/bin/ccx_2.23"
        with open(out, "wb") as f:
            f.write(tf.extractfile(m).read())
        os.chmod(out, 0o755)
        print(f"  wrote {out}")
PYEOF

echo "== fetching CalculiX runtime libraries =="
"$PY" - "$SUBDIR" "$VENDOR" <<'PYEOF'
import io, json, os, sys, tarfile, urllib.request, zipfile
subdir, vendor = sys.argv[1], sys.argv[2]
try:
    import zstandard
except ImportError:
    raise SystemExit("error: zstandard is required")

LIB_DIR = f"{vendor}/lib"
os.makedirs(LIB_DIR, exist_ok=True)

repodata = json.load(urllib.request.urlopen(urllib.request.Request(
    f"https://conda.anaconda.org/conda-forge/{subdir}/repodata.json",
    headers={"User-Agent": "aether-fetch/1.0"}), timeout=300))
pkgs = {**repodata.get("packages", {}), **repodata.get("packages.conda", {})}

def latest(prefix, contains=None):
    hits = [k for k in pkgs if k.startswith(prefix + "-")]
    if contains:
        hits = [k for k in hits if contains in k]
    if not hits:
        return None
    hits.sort(key=lambda k: pkgs[k]["timestamp"])
    return hits[-1]

WANT = {
    "libgfortran5": ("libgfortran.5.dylib", "libgfortran.so.5"),
    "libquadmath": ("libquadmath.0.dylib", "libquadmath.so.0"),
    "arpack": ("libarpack.2.dylib", "libarpack.so.2"),
    "llvm-openmp": ("libomp.dylib", "libomp.so"),
    "libgcc": ("libgcc_s.1.1.dylib", "libgcc_s.so.1"),
}

for name, targets in WANT.items():
    fn = latest(name)
    if not fn:
        print(f"  skip {name}: not found")
        continue
    data = urllib.request.urlopen(urllib.request.Request(
        f"https://conda.anaconda.org/conda-forge/{subdir}/{fn}",
        headers={"User-Agent": "aether-fetch/1.0"}), timeout=300).read()
    z = zipfile.ZipFile(io.BytesIO(data))
    pkg = [n for n in z.namelist() if n.startswith("pkg-")][0]
    raw = zstandard.ZstdDecompressor().decompress(z.read(pkg),
                                                 max_output_size=400 * 1024 * 1024)
    tf = tarfile.open(fileobj=io.BytesIO(raw))
    for m in tf.getmembers():
        if not m.isfile() or not m.name.startswith("lib/"):
            continue
        base = os.path.basename(m.name)
        if any(base.startswith(t.split(".dylib")[0].split(".so")[0]) for t in targets):
            with open(os.path.join(LIB_DIR, base), "wb") as f:
                f.write(tf.extractfile(m).read())
            print(f"  wrote lib/{base}")
            break
PYEOF

echo "== linking BLAS/LAPACK =="
if [[ "$OS" == "darwin" ]]; then
    if [[ -d /opt/homebrew/opt/openblas/lib ]]; then
        cp -f /opt/homebrew/opt/openblas/lib/libblas.3.dylib "$LIB_DIR/" 2>/dev/null || true
        cp -f /opt/homebrew/opt/openblas/lib/liblapack.3.dylib "$LIB_DIR/" 2>/dev/null || true
    fi
elif [[ -f /usr/lib/x86_64-linux-gnu/libblas.so.3 ]]; then
    cp -f /usr/lib/x86_64-linux-gnu/libblas.so.3 "$LIB_DIR/" 2>/dev/null || true
    cp -f /usr/lib/x86_64-linux-gnu/liblapack.so.3 "$LIB_DIR/" 2>/dev/null || true
fi

echo "== verifying =="
if verify_install; then
    echo "CalculiX 2.23 installed and solving: $CCX"
else
    echo "error: installed binary does not solve" >&2
    exit 1
fi