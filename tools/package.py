#!/usr/bin/env python3
"""Packs what Horizon installs (docs/api/mod-files.md), into target/:
  chestcavity.hnmod         the native code only (horizon-native.json + mixins/chestcavity/); goes in horizon-native-mods/
  chestcavity-content.hmod  the content only (items, models, recipes); goes in horizon-mods/
  chestcavity.hmod          both in one file, for a launcher and game that read content from a native .hmod
                            (newer than Nightly 2026-11-08, which does not)
Re-runnable. Check each with `cargo horizon-mod check <file>`."""
import json
import re
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUNDLE = ROOT / "native"
# The Horizon commit it was tested on (Nightly 2026-11-08), the horizon-mixins commit that nightly's
# native build used, and the native hook points' version the mod needs (that nightly's own).
HORIZON = "fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
HOOKS = 411
ID = "chestcavity"


def pack(out, version, content, native):
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        if content:
            z.write(ROOT / "horizon-mod.toml", "horizon-mod.toml")
            for top in ("assets", "data"):
                for p in sorted((ROOT / top).rglob("*")):
                    # Tool caches (.claude-flow, __pycache__) are not content: they can hold local paths.
                    if p.is_file() and not {".claude-flow", "__pycache__"} & set(p.relative_to(ROOT).parts):
                        z.write(p, p.relative_to(ROOT).as_posix())
        if native:
            manifest = {"schema": 1, "id": ID, "version": version, "game": "26.3", "horizon": HORIZON, "mixins": MIXINS, "hooks": HOOKS}
            z.writestr("horizon-native.json", json.dumps(manifest, indent=2, sort_keys=True) + "\n")
            # Only what a bundle may carry: .rs and .toml (no Cargo.lock, no target/).
            for p in sorted(BUNDLE.rglob("*")):
                if p.is_file() and p.suffix in (".rs", ".toml") and "target" not in p.relative_to(BUNDLE).parts:
                    z.write(p, f"mixins/{ID}/" + p.relative_to(BUNDLE).as_posix())
    print(f"{out.name} ({out.stat().st_size} bytes, version {version})")


def main():
    toml = (BUNDLE / "horizon-mixin.toml").read_text(encoding="utf-8")
    version = re.search(r'^version\s*=\s*"([^"]+)"', toml, re.M).group(1)
    assert f'horizon_revision = "{HORIZON}"' in toml, "horizon-mixin.toml pins another Horizon"
    assert f'version = "{version}"' in (ROOT / "horizon-mod.toml").read_text(encoding="utf-8"), "versions differ"
    out = ROOT / "target"
    out.mkdir(exist_ok=True)
    pack(out / f"{ID}.hnmod", version, content=False, native=True)
    pack(out / f"{ID}-content.hmod", version, content=True, native=False)
    pack(out / f"{ID}.hmod", version, content=True, native=True)


if __name__ == "__main__":
    main()
