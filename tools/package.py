#!/usr/bin/env python3
"""Packs target/chestcavity.hmod: the content mod (assets/, data/, horizon-mod.toml) and the native
bundle (horizon-native.json, mixins/chestcavity/) in one file, as docs/api/mod-files.md describes.
Re-runnable. Check it with `cargo horizon-mod check target/chestcavity.hmod`."""
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


def main():
    toml = (BUNDLE / "horizon-mixin.toml").read_text(encoding="utf-8")
    version = re.search(r'^version\s*=\s*"([^"]+)"', toml, re.M).group(1)
    assert f'horizon_revision = "{HORIZON}"' in toml, "horizon-mixin.toml pins another Horizon"
    assert f'version = "{version}"' in (ROOT / "horizon-mod.toml").read_text(encoding="utf-8"), "versions differ"
    native = {"schema": 1, "id": ID, "version": version, "game": "26.3", "horizon": HORIZON, "mixins": MIXINS, "hooks": HOOKS}
    out = ROOT / "target" / f"{ID}.hmod"
    out.parent.mkdir(exist_ok=True)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.write(ROOT / "horizon-mod.toml", "horizon-mod.toml")
        for top in ("assets", "data"):
            for p in sorted((ROOT / top).rglob("*")):
                if p.is_file():
                    z.write(p, p.relative_to(ROOT).as_posix())
        z.writestr("horizon-native.json", json.dumps(native, indent=2, sort_keys=True) + "\n")
        # Only what a bundle may carry: .rs and .toml (no Cargo.lock, no target/).
        for p in sorted(BUNDLE.rglob("*")):
            if p.is_file() and p.suffix in (".rs", ".toml") and "target" not in p.relative_to(BUNDLE).parts:
                z.write(p, f"mixins/{ID}/" + p.relative_to(BUNDLE).as_posix())
    print(f"{out} ({out.stat().st_size} bytes, version {version})")


if __name__ == "__main__":
    main()
