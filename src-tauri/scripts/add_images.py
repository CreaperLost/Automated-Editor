"""Adds a delivered image set to AeroEdits' image library (src-tauri/assets/images).

    python scripts/add_images.py <look>=<drop folder> [<look>=<drop folder> ...]

The library in assets/images is the only copy: its WebP and SVG files and catalog.json. A drop
folder is a new set made elsewhere (see assets/image-looks/README.md for the prompts):

- A style look (light, dark, editorial, neon, or a new one named in LOOKS below): groups.json
  is {"<look>": {"description", "groups": {"<group>": {"description", "terms"}}}} and each
  group folder holds PNGs with a catalog.json
  {"<name>.png": {"depicts", "placement", "flova_resource"}}.
- broadsheet: groups.json is {"<topic>": {"description", "groups": {...}}}, and the group
  folders sit under <topic>/.

A group already in the library gets the new pictures added (a name already there is replaced).
Pictures become WebP, quality 80 with transparency at 70; objects are at most 1024 px on their
long side (full-bleed grounds 2048, sourced pictures 1440); standing figures (768x1024 in a
people, operator, reviewer, executive or humanoid group) are moved so their feet stand at
y=950. Then rebuild the app (build.rs embeds the folder) and run `cargo test --lib image_library`.
Needs Pillow.
"""
import json
import pathlib
import sys

from PIL import Image

LIB = pathlib.Path(__file__).resolve().parent.parent / "assets" / "images"
BASELINE = 950
FULL_BLEED = {"paper-warm", "paper-clipping", "halftone"}
FIGURE_GROUPS = {"operator", "reviewer", "executive", "humanoid", "people"}
# A new look needs an entry here: its ground and accent let styles no look names find it.
LOOKS = {
    "broadsheet": {"description": "Engraved ink illustrations on cream newsprint, for faceless explainers.",
                   "ground": "#F0E9DA", "accent": "#C8321F", "styles": ["Broadsheet"]},
    "light": {"description": "Charcoal monoline product sketches with pale-blue fills, on white.",
              "ground": "#FFFFFF", "accent": "#0071E3", "styles": ["Light", "My library", "Word Sync"]},
    "dark": {"description": "Soft-white monoline sketches with blue accents, on near-black.",
             "ground": "#111114", "accent": "#0A84FF", "styles": ["Dark"]},
    "editorial": {"description": "Two-colour navy and blue print illustrations with grain, on pale paper.",
                  "ground": "#F7F8FC", "accent": "#246BFE", "styles": ["Hermes Editorial"]},
    "neon": {"description": "Chunky pink and violet glossy stickers with white edges, on deep violet.",
             "ground": "#12062E", "accent": "#FF2E93", "styles": ["Neon Pop", "Fancy-Short"]},
}


def convert(src, dst, figure):
    """Writes `src` as the library's WebP (or copies an SVG); answers its size."""
    dst.parent.mkdir(parents=True, exist_ok=True)
    if src.suffix.lower() == ".svg":
        dst.write_bytes(src.read_bytes())
        return None
    im = Image.open(src)
    im.load()
    if im.mode not in ("RGB", "RGBA"):
        im = im.convert("RGBA" if "A" in im.getbands() or "transparency" in im.info else "RGB")
    if figure and im.size == (768, 1024):
        feet = im.getchannel("A").point(lambda v: 255 if v > 20 else 0).getbbox()[3]
        if feet != BASELINE:
            moved = Image.new("RGBA", im.size, (0, 0, 0, 0))
            moved.paste(im.convert("RGBA"), (0, BASELINE - feet))
            im = moved
    longest = 2048 if dst.stem in FULL_BLEED else 1440 if "sourced" in dst.parts else 1024
    if max(im.size) > longest:
        scale = longest / max(im.size)
        im = im.resize((round(im.width * scale), round(im.height * scale)), Image.Resampling.LANCZOS)
    im.save(dst, "WEBP", quality=80, method=6, alpha_quality=70)
    return im.size


def add(catalog, look, drop):
    if look not in LOOKS:
        sys.exit(f"Unknown look {look}: add it to LOOKS first")
    if not any(l["id"] == look for l in catalog["looks"]):
        sourced = [l for l in catalog["looks"] if l["id"] == "sourced"]
        others = [l for l in catalog["looks"] if l["id"] != "sourced"]
        catalog["looks"] = others + [{"id": look, **LOOKS[look]}] + sourced
    added = 0
    for tid, topic in json.loads((drop / "groups.json").read_text(encoding="utf-8")).items():
        if look == "broadsheet":
            topic_id, base, tdir = f"broadsheet/{tid}", f"broadsheet/{tid}", drop / tid
        else:
            if tid != look:
                sys.exit(f"{drop}/groups.json names {tid}, not {look}")
            topic_id, base, tdir = look, look, drop
        t = next((t for t in catalog["topics"] if t["id"] == topic_id), None)
        if t is None:
            t = {"id": topic_id, "look": look, "description": topic["description"], "groups": []}
            catalog["topics"].append(t)
        for gid, group in topic["groups"].items():
            entries = json.loads((tdir / gid / "catalog.json").read_text(encoding="utf-8"))
            prefix = f"{base}/{gid}"
            figure_group = gid in FIGURE_GROUPS
            for name, entry in entries.items():
                src = tdir / gid / name
                stem = src.stem
                ext = ".svg" if src.suffix.lower() == ".svg" else ".webp"
                figure = figure_group or stem == "humanoid-front"
                placement = entry.get("placement", "")
                if figure:
                    placement = placement.replace("y=922", f"y={BASELINE}")
                res = entry.get("flova_resource")
                image = {
                    "id": f"{prefix}/{stem}", "look": look, "file": f"{prefix}/{stem}{ext}",
                    "depicts": entry["depicts"], "placement": placement,
                    "licence": {"kind": "noCredit",
                                "source": "Generated for this library (GPT Image 2.5, Flova project)"
                                          + (f", resource {res}" if res else "")},
                }
                size = convert(src, LIB / image["file"], figure)
                if size:
                    image["width"], image["height"] = size
                catalog["images"] = [i for i in catalog["images"] if i["id"] != image["id"]]
                catalog["images"].append(image)
                added += 1
            g = next((g for g in t["groups"] if g["id"] == gid), None)
            if g is None:
                g = {"id": gid, "description": group["description"], "terms": group.get("terms", []),
                     "sourced": False, "count": 0}
                t["groups"].append(g)
            else:
                g["terms"] += [w for w in group.get("terms", []) if w not in g["terms"]]
            g["count"] = sum(1 for i in catalog["images"] if i["id"].rsplit("/", 1)[0] == f"{topic_id}/{gid}")
    return added


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    path = LIB / "catalog.json"
    catalog = json.loads(path.read_text(encoding="utf-8"))
    for arg in sys.argv[1:]:
        look, _, drop = arg.partition("=")
        n = add(catalog, look.strip(), pathlib.Path(drop))
        print(f"{look}: {n} pictures added")
    names = [(i["look"], i["id"].rsplit("/", 1)[1]) for i in catalog["images"]]
    dup = {x for x in names if names.count(x) > 1}
    if dup:
        sys.exit(f"File names repeat inside a look: {dup}")
    path.write_text(json.dumps(catalog, indent=1, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    total = sum(p.stat().st_size for p in LIB.rglob("*") if p.is_file())
    print(len(catalog["images"]), "pictures,", total // 1024 // 1024, "MB")


if __name__ == "__main__":
    main()
