#!/usr/bin/env bash
# Builds pMARS, the reference Core War simulator, into tools/pmars/.
#
# pMARS is the correctness oracle for CoreWarz's classic dialect. It is not
# packaged for Homebrew, so it is built from source. The build output is
# gitignored; run this once before running the differential tests.
#
# Source: https://github.com/akosela/pmars (pMARS v0.9.3). The originally
# planned mirror, https://github.com/rodrigosetti/pmars, no longer exists.
# See tests/corpus/README.md for the confirmed flag meanings and the two
# local source patches this script applies to get an old C codebase through
# a modern clang on macOS (a broken include guard in src/sim.h, and building
# with no display backend at all instead of the curses/SVGA targets, neither
# of which links on macOS).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
dest="$here/pmars"

if [ -x "$dest/pmars" ]; then
    echo "pmars already built at $dest/pmars"
    exit 0
fi

rm -rf "$dest"
git clone --depth 1 https://github.com/akosela/pmars "$dest/src"

repo_dir="$dest/src"
src_dir="$repo_dir/src"

# Fix a broken include guard in sim.h: `#ifndef SIM_INCLUDED` /
# `#define SIM_INCLUDED` was immediately followed by a stray `#endif`, so
# the guard never protected the header body. That let sim.h be reprocessed
# and produced a hard compile error ("redefinition of round") in disasm.c.
python3 - "$src_dir/sim.h" << 'PYEOF'
import sys
path = sys.argv[1]
with open(path) as f:
    content = f.read()
broken = "#ifndef SIM_INCLUDED\n#define SIM_INCLUDED\n#endif\n"
fixed = "#ifndef SIM_INCLUDED\n#define SIM_INCLUDED\n"
if broken in content:
    content = content.replace(broken, fixed)
    if not content.rstrip().endswith("#endif /* SIM_INCLUDED */"):
        content = content.rstrip("\n") + "\n\n#endif /* SIM_INCLUDED */\n"
    with open(path, "w") as f:
        f.write(content)
PYEOF

# Build with no display backend at all: the Makefile's default target wants
# SVGA libraries (not available on macOS) and its `curses` target does not
# compile against modern ncurses headers here. Without any *GRAPHX macro,
# sim.c skips all display code and compiles the plain batch simulator,
# which is all headless differential testing needs.
make -C "$repo_dir" clean
make -C "$repo_dir" CC=cc \
    CFLAGS="-O -DEXT94 -DPERMUTATE -fno-builtin-round -Wno-implicit-function-declaration -Wno-deprecated-non-prototype" \
    LIB=""

mkdir -p "$dest"
cp "$src_dir/pmars" "$dest/pmars"
echo "built $dest/pmars"
"$dest/pmars" -h 2>&1 | head -1 || true
