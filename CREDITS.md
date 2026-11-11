# Credits

This mod is a port to Horizon. Its textures, organ names, organ scores, chest cavity
types and recipes come from three Minecraft mods; `tools/import.py` copies them from
`vendor/` (the upstream repositories, at the commits below).

| Source | Author | Commit | License | What comes from it |
|---|---|---|---|---|
| [Chest Cavity](https://github.com/Tigereye504/chestcavity) (branch `1.20.1`) | Tigereye504 | `3f28dfa` | CC0-1.0 (`fabric.mod.json`) | most organs, food, cleavers, the Chest Opener; organ and type data; recipes; the score rules in `src/effects.rs` |
| [Chest Cavity Beyond](https://github.com/zhaijineet/Chest-Cavity-Beyond) | zhaijineet | `84a7a1e` | MIT, see `licenses/chest-cavity-beyond-MIT.txt` | basalt, elder, sculk, breeze, slime, magma and other new organs, their scores and types |
| [Chest Cavity Astro Adds](https://github.com/A5TR0spud/chest-cavity-astro-adds) | A5TR0spud | `5f31a0c` | CC0-1.0 in its `LICENSE` file, but CC BY-NC-SA 4.0 in its `fabric.mod.json` | the add-on organs (abyssal, steel, copper, gold, tumors, canisters, ...) and their recipes |

Astro Adds' two licenses disagree, so its textures (`assets/chestcavity/textures/item/`, the
items `data/chestcavity/horizon/item/` lists from it) are treated as CC BY-NC-SA 4.0: credited
to A5TR0spud, not for commercial use, shared under the same license.
