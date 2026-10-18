#!/usr/bin/env python3
"""Refresh gallery/photos.json from Alice's public Flickr photostream.

The photostream pages expose 25 photos each, but /photos/<nsid>/with/<id>/
returns the whole 100-photo window around a photo. Walking those windows
enumerates the entire stream without an API key.

Palettes are computed from a 500px thumbnail with Pillow. Camera/lens data
is only available through the Flickr API, so it is enriched when
FLICKR_API_KEY is present and --exif is passed.

Usage:
    python tools/gallery/sync.py            # refresh + palettes
    python tools/gallery/sync.py --no-palette
    FLICKR_API_KEY=... python tools/gallery/sync.py --exif
"""
from __future__ import annotations

import argparse
import colorsys
import io
import json
import os
import re
import sys
import time
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PHOTOS_PATH = ROOT / "gallery" / "photos.json"
NSID = "185219841@N08"
UA = {"User-Agent": "Mozilla/5.0 (compatible; alicemow-gallery/1.0)"}
WIDTHS = {"s": 240, "m": 500, "z": 640, "c": 800, "b": 1024, "l": 1600, "h": 2048}


def log(message: str) -> None:
    print(f"[gallery] {message}", file=sys.stderr)


def get(url: str, timeout: int = 40) -> bytes:
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return response.read()


def model_export(html: str):
    marker = "modelExport:"
    index = html.find(marker)
    if index == -1:
        return None
    segment = html[index + len(marker):]
    depth = 0
    for i, char in enumerate(segment):
        if char == "{":
            if depth == 0:
                start = i
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return json.loads(segment[start:i + 1])
    return None


def normalise(url: str | None) -> str | None:
    if not url:
        return None
    return "https:" + url if url.startswith("//") else url


def size_url(sizes: dict, key: str) -> str | None:
    try:
        return normalise(sizes["data"][key]["data"]["displayUrl"])
    except Exception:  # noqa: BLE001
        return None


def fetch_batch(seed: str):
    html = get(f"https://www.flickr.com/photos/{NSID}/with/{seed}/").decode("utf-8", "replace")
    export = model_export(html)
    if not export:
        return [], 0
    page = export["main"]["photostream-models"][0]["data"]["photoPageList"]["data"]
    total = int(page.get("totalItems") or 0)
    records = []
    for model in page.get("_data") or []:
        if not isinstance(model, dict):
            continue
        data = model.get("data") or {}
        if not data.get("id"):
            continue
        stats = (data.get("stats") or {}).get("data") or {}
        sizes = data.get("sizes") or {}
        records.append({
            "id": str(data["id"]),
            "secret": data.get("secret"),
            "title": data.get("title") or "",
            "date": stats.get("dateTaken") or "",
            "width": data.get("oWidth"),
            "height": data.get("oHeight"),
            "sizes": {
                key: url
                for key in WIDTHS
                if (url := size_url(sizes, key))
            },
        })
    return records, total


def load_existing():
    if not PHOTOS_PATH.exists():
        return {}
    try:
        data = json.loads(PHOTOS_PATH.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001
        log(f"could not read {PHOTOS_PATH.name}: {exc}")
        return {}
    return {str(p["id"]): p for p in data.get("photos", []) if p.get("id")}


def public_feed():
    url = (
        "https://www.flickr.com/services/feeds/photos_public.gne"
        f"?id={NSID}&format=json&nojsoncallback=1"
    )
    try:
        return json.loads(get(url).decode("utf-8", "replace")).get("items") or []
    except Exception as exc:  # noqa: BLE001
        log(f"public feed unavailable: {exc}")
        return []


def crawl(existing: dict, feed: list) -> dict:
    photos: dict[str, dict] = {}
    seeds: list[str] = []
    total = 0

    if existing:
        ordered = sorted(existing.values(), key=lambda p: (p.get("date") or "", p["id"]))
        seeds.extend([ordered[-1]["id"], ordered[len(ordered) // 2]["id"], ordered[0]["id"]])
    if feed:
        match = re.search(r"/photos/[^/]+/(\d+)/", feed[0].get("link", ""))
        if match:
            seeds.insert(0, match.group(1))

    tried: set[str] = set()
    while seeds:
        seed = seeds.pop(0)
        if seed in tried:
            continue
        tried.add(seed)
        try:
            batch, batch_total = fetch_batch(seed)
        except Exception as exc:  # noqa: BLE001
            log(f"batch {seed} failed: {exc}")
            continue
        total = max(total, batch_total)
        fresh = 0
        for record in batch:
            if record["id"] not in photos:
                photos[record["id"]] = record
                fresh += 1
        log(f"seed {seed}: {len(batch)} models, {fresh} new, {len(photos)} collected")
        if fresh and len(photos) < total:
            # The window's edges point at the neighbouring windows.
            for candidate in (batch[0]["id"], batch[len(batch) // 2]["id"], batch[-1]["id"]):
                if candidate not in tried and candidate not in seeds:
                    seeds.append(candidate)
        if total and len(photos) >= total:
            break

    if total and len(photos) < total:
        log(f"only {len(photos)}/{total} photos reached")
    return photos


def palette(record: dict) -> dict:
    url = record["sizes"].get("m") or record["sizes"].get("z") or record["sizes"].get("s")
    if not url:
        return {}
    try:
        from PIL import Image
    except ImportError:
        log("Pillow is not installed; skipping palettes (pip install pillow)")
        return {}
    try:
        image = Image.open(io.BytesIO(get(url, timeout=30))).convert("RGB")
        image.thumbnail((80, 80))
        quantised = image.quantize(colors=5, method=Image.Quantize.MEDIANCUT).convert("RGB")
        counts: dict[tuple[int, int, int], int] = {}
        pixels = (
            quantised.get_flattened_data()
            if hasattr(quantised, "get_flattened_data")
            else quantised.getdata()
        )
        for pixel in pixels:
            counts[pixel] = counts.get(pixel, 0) + 1
        total = max(1, sum(counts.values()))
        top = sorted(counts.items(), key=lambda item: -item[1])[:4]
        colours = ["#%02x%02x%02x" % rgb for rgb, _ in top]

        best = None
        for rgb, count in top:
            r, g, b = (value / 255 for value in rgb)
            hue, saturation, value = colorsys.rgb_to_hsv(r, g, b)
            score = count / total * saturation
            if best is None or score > best[0]:
                best = (score, hue * 360, saturation, value)
        _, hue, saturation, value = best
        return {
            "palette": colours,
            "hue": round(hue, 1),
            "sat": round(saturation, 3),
            "val": round(value, 3),
        }
    except Exception as exc:  # noqa: BLE001
        log(f"palette {record['id']} failed: {exc}")
        return {}


def enrich_exif(photos: dict, key: str) -> None:
    missing = [p for p in photos.values() if not p.get("exif")]
    log(f"EXIF: enriching {len(missing)} photos")
    for index, record in enumerate(missing, 1):
        query = urllib.parse.urlencode({
            "method": "flickr.photos.getExif",
            "api_key": key,
            "photo_id": record["id"],
            "format": "json",
            "nojsoncallback": "1",
        })
        try:
            payload = json.loads(get(f"https://www.flickr.com/services/rest/?{query}").decode("utf-8"))
            photo = payload.get("photo") or {}
            exif: dict[str, str] = {}
            if photo.get("camera"):
                exif["camera"] = photo["camera"]
            for entry in photo.get("exif") or []:
                tag = entry.get("tag") or ""
                raw = entry.get("raw") or entry.get("clean") or {}
                value = raw.get("_content") if isinstance(raw, dict) else raw
                if not value:
                    continue
                if tag in ("Model", "LensModel", "Lens") and "camera" not in exif:
                    exif["camera"] = str(value)
                elif tag == "LensModel":
                    exif["lens"] = str(value)
                elif tag == "FocalLength":
                    exif["focal"] = f"{value}mm"
                elif tag == "FNumber":
                    exif["aperture"] = f"f/{value}"
                elif tag == "ExposureTime":
                    exif["shutter"] = f"{value}s"
                elif tag == "ISO":
                    exif["iso"] = str(value)
            if exif:
                record["exif"] = exif
        except Exception as exc:  # noqa: BLE001
            log(f"EXIF {record['id']} failed: {exc}")
        if index % 25 == 0:
            log(f"EXIF: {index}/{len(missing)}")
        time.sleep(0.12)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-palette", action="store_true", help="skip colour analysis")
    parser.add_argument("--exif", action="store_true", help="enrich camera data via the Flickr API")
    args = parser.parse_args()

    existing = load_existing()
    feed = public_feed()

    feed_by_id = {}
    for item in feed:
        match = re.search(r"/photos/[^/]+/(\d+)/", item.get("link", ""))
        if match:
            description = re.sub(r"<[^>]+>", " ", item.get("description") or "")
            feed_by_id[match.group(1)] = {
                "feed_tags": [t for t in (item.get("tags") or "").split() if t],
                "feed_description": re.sub(r"\s+", " ", description).strip(),
            }

    discovered = crawl(existing, feed)
    if not discovered:
        log("nothing fetched; keeping the existing archive")
        return 0

    merged = {}
    for pid, record in discovered.items():
        previous = existing.get(pid, {})
        record["feed_tags"] = previous.get("feed_tags") or feed_by_id.get(pid, {}).get("feed_tags", [])
        record["feed_description"] = (
            previous.get("feed_description") or feed_by_id.get(pid, {}).get("feed_description", "")
        )
        for field in ("palette", "hue", "sat", "val", "exif"):
            if previous.get(field) is not None:
                record[field] = previous[field]
        merged[pid] = record

    if not args.no_palette:
        try:
            import PIL  # noqa: F401
        except ImportError:
            log("Pillow is not installed; skipping palettes (pip install pillow)")
            args.no_palette = True

    if not args.no_palette:
        todo = [pid for pid, record in merged.items() if not record.get("palette")]
        log(f"palettes: {len(todo)} to compute")
        with ThreadPoolExecutor(max_workers=8) as pool:
            for pid, info in zip(todo, pool.map(lambda pid: palette(merged[pid]), todo)):
                merged[pid].update(info)

    if args.exif:
        key = os.environ.get("FLICKR_API_KEY")
        if key:
            enrich_exif(merged, key)
        else:
            log("--exif given but FLICKR_API_KEY is unset; skipping")

    records = sorted(merged.values(), key=lambda p: (p.get("date") or "", p["id"]), reverse=True)
    PHOTOS_PATH.write_text(
        json.dumps(
            {
                "generated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "owner": NSID,
                "photos": records,
            },
            indent=1,
            ensure_ascii=False,
        ) + "\n",
        encoding="utf-8",
    )
    log(f"wrote {len(records)} photos to {PHOTOS_PATH.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
