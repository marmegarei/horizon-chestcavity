"""Imports Chest Cavity, Chest Cavity Beyond and Astro Adds from vendor/ into
assets/, data/ and src/organs.rs. Re-runnable: it rewrites everything it owns.

    python tools/import.py
"""
import json
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
V = ROOT / "vendor"
CC = V / "chestcavity/src/main"
CC_RES = CC / "resources"
ASTRO = V / "astro/src/main"
BEYOND = V / "beyond/src/main"
BEYOND_GEN = V / "beyond/src/generated/resources"
NS = "chestcavity"

OUT_ASSETS = ROOT / "assets" / NS
OUT_DATA = ROOT / "data"
OUT_RS = ROOT / "src/organs.rs"

# Beyond renamed some of the original's organs; theirs map back to ours.
BEYOND_ALIAS = {
    "gill": "gills", "small_gill": "small_gills", "gas_sac": "gas_bladder",
    "golem_armor_plate": "golem_plating", "writhing_soul_sand": "writhing_soulsand",
}
# Beyond items that are not organs or have no use here.
BEYOND_SKIP = {"chest_opener", "biological_analyzer", "portable_jukebox", "hotbar"}
# Mobs whose organs come from Beyond's types (newer mobs, or Beyond's own organs).
BEYOND_MOBS = {
    "minecraft:warden": "warden", "minecraft:breeze": "breeze", "minecraft:slime": "slime",
    "minecraft:magma_cube": "magma_cube", "minecraft:strider": "strider",
    "minecraft:sniffer": "brute_animal", "minecraft:guardian": "guardian",
    "minecraft:elder_guardian": "elder_guardian", "minecraft:snow_golem": "snow_golem",
}
# 26.3 mobs the 1.20 data doesn't know, by the original's type they resemble.
EXTRA_MOBS = {
    "minecraft:bogged": "undead/skeleton", "minecraft:camel": "animals/herbivore",
    "minecraft:armadillo": "animals/small_herbivore", "minecraft:happy_ghast": "magical_beasts/ghast",
    "minecraft:copper_golem": "constructs/iron_golem", "minecraft:parched": "undead/skeleton",
}
CLEAVER_TEMPLATES = {"wooden": "wooden_sword", "stone": "stone_sword", "gold": "golden_sword",
                     "iron": "iron_sword", "diamond": "diamond_sword", "netherite": "netherite_sword"}

warnings = []


def warn(msg):
    warnings.append(msg)


def load(p):
    return json.loads(Path(p).read_text(encoding="utf-8"))


def dump(p, obj):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


# ---------------------------------------------------------------- items

items = {}  # our id (no namespace) -> dict(texture=Path, template, stack, handheld, name, source)


def java_registry(path, reg_re):
    """name -> (maxCount, food expression or '') from a Java items class."""
    src = Path(path).read_text(encoding="utf-8")
    consts = {}
    for m in re.finditer(r"public static final \w+ (\w+) = new (.*?);\s*$", src, re.M):
        consts[m.group(1)] = m.group(2)
    out = {}
    for m in re.finditer(reg_re, src, re.S):
        name, rest = m.group(1), m.group(2)
        decl = consts.get(rest.strip(), rest)
        mc = re.search(r"maxCount\((\d+)\)", decl)
        food = re.search(r"\.food\(([\w.]+)\)", decl)
        out[name] = (int(mc.group(1)) if mc else None, food.group(1) if food else "")
    return out


def template_for(name, stack, food):
    if name == "chest_opener":
        return "minecraft:stick", 1
    if name.endswith("_cleaver"):
        mat = name[: -len("_cleaver")]
        return "minecraft:" + CLEAVER_TEMPLATES.get(mat, "iron_sword"), None
    if food:
        if "ROTTEN" in food or "TOXIC" in food or "rotten" in name or "toxic" in name:
            return "minecraft:rotten_flesh", stack
        if name.startswith(("cooked_", "burnt_")):
            return "minecraft:cooked_beef", stack
        return "minecraft:beef", stack
    return "minecraft:paper", stack


def model_texture(model_path):
    """The texture a 1.20 item model's layer0 names, and whether it's handheld."""
    m = load(model_path)
    tex = m.get("textures", {}).get("layer0")
    return tex, "handheld" in m.get("parent", "")


def add_items_original():
    reg = java_registry(next(CC.rglob("CCItems.java")), r'registerItem\("(\w+)",\s*(\w+)\)')
    lang = load(CC_RES / f"assets/{NS}/lang/en_us.json")
    for name, (stack, food) in reg.items():
        model = CC_RES / f"assets/{NS}/models/item/{name}.json"
        if not model.exists():
            warn(f"original: no model for {name}")
            continue
        tex, handheld = model_texture(model)
        ns, path = tex.split(":")
        png = CC_RES / f"assets/{ns}/textures/{path}.png"
        if not png.exists():
            warn(f"original: {name} uses {tex}, not shipped")
            continue
        template, stack = template_for(name, stack or (64 if food else 1), food)
        items[name] = dict(texture=png, template=template, stack=stack, handheld=handheld,
                           name=lang.get(f"item.{NS}.{name}", title(name)), source="original")


def add_items_astro():
    reg = java_registry(next(ASTRO.rglob("CCAAItems.java")), r'registerItem\("(\w+)",\s*(new .*?\)\));')
    lang = load(ASTRO / "resources/assets/ccastroadds/lang/en_us.json")
    for name, (stack, food) in reg.items():
        png = ASTRO / f"resources/assets/ccastroadds/textures/item/{name}.png"
        if not png.exists():
            model = ASTRO / f"resources/assets/ccastroadds/models/item/{name}.json"
            if model.exists():
                tex, _ = model_texture(model)
                png = ASTRO / f"resources/assets/{tex.split(':')[0]}/textures/{tex.split(':')[1]}.png"
        if not png.exists():
            warn(f"astro: no texture for {name}")
            continue
        our = astro_id(name)
        template, stack = template_for(name, stack or 1, food)
        items[our] = dict(texture=png, template=template, stack=stack, handheld=False,
                          name=lang.get(f"item.ccastroadds.{name}", title(name)), source="astro")


def astro_id(name):
    return name if name not in ORIGINAL_NAMES else "astro_" + name


def beyond_id(name):
    if name in BEYOND_ALIAS:
        return BEYOND_ALIAS[name]
    if name.startswith("arthropod_"):
        return "insect_" + name[len("arthropod_"):]
    if name in ORIGINAL_NAMES:
        return name
    return "beyond_" + name if items.get(name, {}).get("source") == "astro" else name


BEYOND_SRC = ""
BEYOND_REG = {}  # beyond name -> register block text


def add_items_beyond():
    global BEYOND_SRC
    BEYOND_SRC = next(BEYOND.rglob("InitItem.java")).read_text(encoding="utf-8")
    blocks = re.split(r"public static final Supplier<Item> \w+ = ITEM\.register\(", BEYOND_SRC)[1:]
    lang = load(BEYOND_GEN / "assets/chestcavitybeyond/lang/en_us.json")
    for b in blocks:
        name = re.match(r'\s*"(\w+)"', b).group(1)
        BEYOND_REG[name] = b
        if name in BEYOND_SKIP:
            continue
        our = beyond_id(name)
        if our in ORIGINAL_NAMES:
            continue  # the original's own organ, already there
        png = BEYOND / f"resources/assets/chestcavitybeyond/textures/item/{name}.png"
        if not png.exists():
            warn(f"beyond: no texture for {name}")
            continue
        stack = re.search(r"stacksTo\((\d+)\)", b)
        stack = int(stack.group(1)) if stack else 1
        food = "food" if "FoodProperties" in b or ".food(" in b else ""
        template, stack = template_for(name, stack, food)
        items[our] = dict(texture=png, template=template, stack=stack, handheld=False,
                          name=lang.get(f"item.chestcavitybeyond.{name}", title(name)), source="beyond")


def title(name):
    return " ".join(w.capitalize() for w in name.split("_"))


# ---------------------------------------------------------------- organ scores

organs = {}  # full item id -> dict(scores={score: value}, pseudo=bool)


def score_id(s):
    return s.split(":")[-1]


def add_organ_json(path, remap):
    d = load(path)
    item = remap(d["itemID"])
    if item is None:
        return
    organs[item] = dict(
        scores={score_id(s["id"]): float(s["value"]) for s in d.get("organScores", [])},
        pseudo=bool(d.get("pseudoOrgan")))


def remap_item(full):
    """An upstream item id -> ours, or None if we don't have it."""
    ns, name = full.split(":") if ":" in full else ("minecraft", full)
    if ns == "minecraft" or ns == "c":
        return f"{ns}:{name}"
    if ns == NS:
        return f"{NS}:{name}" if name in items else None
    if ns == "ccastroadds":
        our = astro_id(name)
        return f"{NS}:{our}" if our in items else None
    if ns == "chestcavitybeyond":
        our = beyond_id(name)
        return f"{NS}:{our}" if our in items else None
    return None


BEYOND_ATTR = {"LUCK": "luck", "ARMOR": "defense", "MAX_HEALTH": "health"}


def beyond_scores(block):
    scores = {}
    for m in re.finditer(r"add\w*Attribute\(\s*(?:InitAttribute|Attributes)\.(\w+),\s*(-?[\d.]+)", block):
        attr, val = m.group(1), float(m.group(2))
        key = BEYOND_ATTR.get(attr, attr.lower())
        scores[key] = scores.get(key, 0) + val
    return scores


def add_organs():
    for p in sorted((CC_RES / f"data/{NS}/organs").rglob("*.json")):
        add_organ_json(p, remap_item)
    for p in sorted((ASTRO / "resources/data/ccastroadds/organs").rglob("*.json")):
        add_organ_json(p, remap_item)
    for name, block in BEYOND_REG.items():
        our = remap_item(f"chestcavitybeyond:{name}")
        if our and our not in organs and "Organ.builder()" in block:
            organs[our] = dict(scores=beyond_scores(block), pseudo=False)


# ---------------------------------------------------------------- chest cavity types

kinds = {}  # name -> dict(slots=[(pos, item, count)], base={}, drop=float, boss=bool)


def stack_of(full):
    name = full.split(":")[1]
    if full.startswith(NS + ":") and name in items:
        return items[name]["stack"] or 1
    return 64  # vanilla pseudo-organs: a full stack scores 1


def add_kinds_original():
    tdir = CC_RES / f"data/{NS}/types"
    for p in sorted(tdir.rglob("*.json")):
        rel = p.relative_to(tdir).with_suffix("").as_posix()
        if rel.startswith("compatibility") or rel == "example":
            continue
        d = load(p)
        slots = []
        for e in d.get("defaultChestCavity", []):
            item = remap_item(e["item"])
            if item is None:
                warn(f"type {rel}: unknown item {e['item']}")
                continue
            slots.append((e["position"], item, e.get("count", stack_of(item))))
        kinds[rel] = dict(slots=slots,
                          base={score_id(s["id"]): float(s["value"]) for s in d.get("baseOrganScores", [])},
                          drop=float(d.get("dropRateMultiplier", 1)), boss=bool(d.get("bossChestCavity")))


def add_kinds_beyond():
    src = next(BEYOND.rglob("ChestCavityTypeManager.java")).read_text(encoding="utf-8")
    for m in re.finditer(r'register\(ChestCavityBeyond\.of\("(\w+)"\)\)(.*?);\n', src, re.S):
        name, body = m.group(1), m.group(2)
        base = re.search(r"copyWith\((\w+)\)", body)
        slots = list(kinds.get("beyond/" + base.group(1).lower(), {}).get("slots", [])) if base else []
        for r in re.finditer(r"set(First|Second|Third)Row\((\d+),\s*(InitItem|Items)\.(\w+)", body):
            pos = {"First": 0, "Second": 9, "Third": 18}[r.group(1)] + int(r.group(2))
            ns = "chestcavitybeyond" if r.group(3) == "InitItem" else "minecraft"
            item = remap_item(f"{ns}:{r.group(4).lower()}")
            if item:
                slots = [x for x in slots if x[0] != pos] + [(pos, item, stack_of(item))]
        if slots:
            kinds["beyond/" + name] = dict(slots=slots, base={}, drop=1.0,
                                           boss=name in ("ender_dragon", "wither", "warden"))


mobs = {}  # entity id -> kind name


def add_mobs():
    adir = CC_RES / f"data/{NS}/entity_assignment"
    for p in sorted(adir.rglob("*.json")):
        if p.relative_to(adir).as_posix().startswith("compatibility"):
            continue
        d = load(p)
        kind = d["chestcavity"].split("types/")[1].removesuffix(".json")
        for e in d["entities"]:
            if e.startswith("minecraft:") and kind in kinds:
                mobs[e] = kind
    for e, kind in EXTRA_MOBS.items():
        mobs.setdefault(e, kind)
    for e, kind in BEYOND_MOBS.items():
        if "beyond/" + kind in kinds:
            mobs[e] = "beyond/" + kind
        else:
            warn(f"beyond has no type {kind} for {e}")
    mobs.pop("minecraft:player", None)


# ---------------------------------------------------------------- recipes and tags

recipes_skipped = []


def ingredient(i):
    """A 1.20 ingredient (object or list) -> a 26.3 one (string or list)."""
    if isinstance(i, list):
        out = [ingredient(x) for x in i]
        out = [x for x in out if x]
        return out if len(out) > 1 else (out[0] if out else None)
    if isinstance(i, str):
        i = {"tag": i[1:]} if i.startswith("#") else {"item": i}
    if "tag" in i:
        ns, path = i["tag"].split(":")
        return "#" + (f"{NS}:{path}" if ns in (NS, "ccastroadds", "chestcavitybeyond") else i["tag"])
    if "item" not in i:
        return None
    return remap_item(i["item"])


def result(r):
    if isinstance(r, str):
        r = {"item": r}
    item = remap_item(r.get("item") or r.get("id"))
    if not item:
        return None
    out = {"id": item}
    if r.get("count", 1) != 1:
        out["count"] = r["count"]
    return out


def convert_recipe(d):
    t = d["type"]
    if t == "minecraft:crafting_shaped":
        key = {k: ingredient(v) for k, v in d["key"].items()}
        res = result(d["result"])
        if None in key.values() or not res:
            return None
        return {"type": t, "category": "misc", "pattern": d["pattern"], "key": key, "result": res}
    if t == "minecraft:crafting_shapeless":
        ings = [ingredient(x) for x in d["ingredients"]]
        res = result(d["result"])
        if None in ings or not res:
            return None
        return {"type": t, "category": "misc", "ingredients": ings, "result": res}
    if t in ("minecraft:smelting", "minecraft:smoking", "minecraft:campfire_cooking", "minecraft:blasting"):
        ing, res = ingredient(d["ingredient"]), result(d["result"])
        if not ing or not res:
            return None
        return {"type": t, "category": "food" if t != "minecraft:blasting" else "misc", "ingredient": ing,
                "result": res, "experience": d.get("experience", 0.1), "cookingtime": d.get("cookingtime", 200)}
    if t == "chestcavity:crafting_salvage":
        ing = ingredient(d["ingredient"])
        res = result({"item": d["result"], "count": d.get("count", 1)})
        if not ing or not res:
            return None
        # ponytail: one organ per craft; the original salvages a whole stack's worth at once.
        return {"type": "minecraft:crafting_shapeless", "category": "misc", "ingredients": [ing], "result": res}
    return None


def add_recipes():
    out = OUT_DATA / NS / "recipe"
    count = 0
    for prefix, rdir in (("", CC_RES / f"data/{NS}/recipes"),
                         ("astro_", ASTRO / "resources/data/ccastroadds/recipes"),
                         ("beyond_", BEYOND_GEN / "data/chestcavitybeyond/recipe")):
        for p in sorted(rdir.glob("*.json")):
            d = load(p)
            r = convert_recipe(d)
            if r is None:
                recipes_skipped.append(f"{prefix}{p.stem} ({d['type']})")
                continue
            dump(out / f"{prefix}{p.stem}.json", r)
            count += 1
    return count


def add_tags():
    out = OUT_DATA / NS / "tags/item"
    count = 0
    for tdir in (CC_RES / f"data/{NS}/tags/items",):
        for p in sorted(tdir.rglob("*.json")):
            vals = []
            for v in load(p)["values"]:
                v = v["id"] if isinstance(v, dict) else v
                if v.startswith("#"):
                    ns, path = v[1:].split(":")
                    vals.append(f"#{NS}:{path}" if ns == NS else v)
                else:
                    item = remap_item(v)
                    if item:
                        vals.append(item)
            dump(out / p.relative_to(tdir), {"values": vals})
            count += 1
    return count


# ---------------------------------------------------------------- write

def write_items():
    for d in ("textures/item", "models/item", "items"):
        shutil.rmtree(OUT_ASSETS / d, ignore_errors=True)
    shutil.rmtree(OUT_DATA / NS / "horizon/item", ignore_errors=True)
    lang = {}
    for name, it in sorted(items.items()):
        tex = OUT_ASSETS / f"textures/item/{name}.png"
        tex.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(it["texture"], tex)
        parent = "minecraft:item/handheld" if it["handheld"] else "minecraft:item/generated"
        dump(OUT_ASSETS / f"models/item/{name}.json", {"parent": parent, "textures": {"layer0": f"{NS}:item/{name}"}})
        dump(OUT_ASSETS / f"items/{name}.json", {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}})
        decl = {"template": it["template"]}
        if it["stack"]:
            decl["max_stack_size"] = it["stack"]
        dump(OUT_DATA / NS / f"horizon/item/{name}.json", decl)
        lang[f"item.{NS}.{name}"] = it["name"]
    dump(OUT_ASSETS / "lang/en_us.json", lang)


def rs_str(s):
    return json.dumps(s)


def rs_scores(scores):
    return "&[" + ", ".join(f"({rs_str(k)}, {v:g}_f32)" for k, v in sorted(scores.items()) if v) + "]"


def write_rust():
    names = {f"{NS}:{n}": it["name"] for n, it in items.items()}
    lines = ["// Generated by tools/import.py from Chest Cavity, Chest Cavity Beyond and Astro Adds: do not edit.",
             "use crate::cavity::{Kind, Organ};", "",
             "/// Every organ and pseudo-organ, by id.", "pub static ORGANS: &[Organ] = &["]
    for item, o in sorted(organs.items()):
        stack = stack_of(item)
        name = names.get(item, title(item.split(":")[1]))
        lines.append(f"    Organ {{ id: {rs_str(item)}, name: {rs_str(name)}, stack: {stack}, "
                     f"scores: {rs_scores(o['scores'])} }},")
    lines += ["];", "", "/// Every chest cavity type, by name.", "pub static KINDS: &[Kind] = &["]
    order = sorted(kinds)
    for k in order:
        lines.append(f"    // {k}")
        d = kinds[k]
        slots = ", ".join(f"({p}, {rs_str(i)}, {c})" for p, i, c in sorted(d["slots"]))
        lines.append(f"    Kind {{ slots: &[{slots}], base: {rs_scores(d['base'])}, "
                     f"drop_rate: {d['drop']:g}_f32, boss: {str(d['boss']).lower()} }},")
    lines += ["];", "", "/// Mobs that drop organs, with their type's index in KINDS.",
              "pub static MOBS: &[(&str, usize)] = &["]
    for e, k in sorted(mobs.items()):
        lines.append(f"    ({rs_str(e)}, {order.index(k)}),")
    lines += ["];", "", f"/// The player's type.", f"pub const PLAYER: usize = {order.index('humanoids/player')};", ""]
    OUT_RS.write_text("\n".join(lines), encoding="utf-8", newline="\n")


def main():
    add_items_original()
    global ORIGINAL_NAMES
    ORIGINAL_NAMES = set(items)
    add_items_astro()
    add_items_beyond()
    add_organs()
    add_kinds_original()
    add_kinds_beyond()
    add_mobs()
    write_items()
    shutil.rmtree(OUT_DATA / NS / "recipe", ignore_errors=True)
    shutil.rmtree(OUT_DATA / NS / "tags", ignore_errors=True)
    n_recipes = add_recipes()
    n_tags = add_tags()
    write_rust()
    by_source = {}
    for it in items.values():
        by_source[it["source"]] = by_source.get(it["source"], 0) + 1
    print(f"items {len(items)} {by_source}, organs {len(organs)}, kinds {len(kinds)}, mobs {len(mobs)}, "
          f"recipes {n_recipes} (skipped {len(recipes_skipped)}), tags {n_tags}")
    if recipes_skipped:
        print("skipped recipes:", ", ".join(recipes_skipped))
    for w in warnings:
        print("warning:", w)


ORIGINAL_NAMES = set()

if __name__ == "__main__":
    sys.exit(main())
