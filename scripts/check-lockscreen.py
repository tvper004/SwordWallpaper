#!/usr/bin/env python3
"""Check Aerial assignments, media and running players; this is not a visual playback test."""

import json
import datetime
import filecmp
import pathlib
import plistlib
import struct
import subprocess
import sys


SUPPORT = pathlib.Path.home() / "Library/Application Support"
AERIALS = SUPPORT / "com.apple.wallpaper/aerials"
AURA = SUPPORT / "Aura/LoginWallpapers"
INDEX = SUPPORT / "com.apple.wallpaper/Store/Index.plist"
PROVIDER = "com.apple.wallpaper.choice.aerials"


def active_displays():
    source = (
        'import AppKit; import CoreGraphics; '
        'for s in NSScreen.screens { '
        'let n=s.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as! NSNumber; '
        'let u=CGDisplayCreateUUIDFromDisplayID(n.uint32Value)!.takeRetainedValue(); '
        'print(CFUUIDCreateString(nil,u)!) }'
    )
    return subprocess.check_output(["swift", "-e", source], text=True).splitlines()


def boxes(file, start, end):
    offset = start
    while offset + 8 <= end:
        file.seek(offset)
        header = file.read(16)
        size, kind = struct.unpack(">I4s", header[:8])
        header_size = 8
        if size == 1:
            size = struct.unpack(">Q", header[8:16])[0]
            header_size = 16
        elif size == 0:
            size = end - offset
        if size < header_size or offset + size > end:
            raise ValueError("Invalid MP4 box")
        yield kind.decode("ascii", "replace"), offset, size, header_size
        offset += size


def child(file, parent, name):
    _, offset, size, header_size = parent
    return next((box for box in boxes(file, offset + header_size, offset + size) if box[0] == name), None)


def temporal_groups(path):
    required = {"sgpd tscl", "sgpd tsas", "csgm tscl", "csgm tsas"}
    found = set()
    with path.open("rb") as file:
        root = ("file", 0, path.stat().st_size, 0)
        moov = child(file, root, "moov")
        if moov is None:
            return found
        for trak in boxes(file, moov[1] + moov[3], moov[1] + moov[2]):
            if trak[0] != "trak":
                continue
            mdia = child(file, trak, "mdia")
            minf = child(file, mdia, "minf") if mdia else None
            stbl = child(file, minf, "stbl") if minf else None
            if stbl is None:
                continue
            for box in boxes(file, stbl[1] + stbl[3], stbl[1] + stbl[2]):
                if box[0] not in ("sgpd", "csgm"):
                    continue
                file.seek(box[1] + 12)
                group = file.read(4).decode("ascii", "replace")
                found.add(f"{box[0]} {group}")
    return found


def main():
    slots = json.loads((AURA / "slots.json").read_text())
    index = plistlib.loads(INDEX.read_bytes())
    if index.get("AllSpacesAndDisplays") != "$null":
        raise RuntimeError("A global macOS wallpaper overrides Aura's per-display choices")
    manifest = json.loads((AERIALS / "manifest/entries.json").read_text())
    manifest_ids = {entry["id"].upper() for entry in manifest["assets"] if "id" in entry}
    displays = active_displays()
    if not displays:
        raise RuntimeError("macOS did not report any active displays")

    for display in displays:
        slot = slots.get(display)
        if not slot:
            raise RuntimeError(f"No Aura slot is saved for active display {display}")
        display_config = index.get("Displays", {}).get(display, {})
        for mode_name in ("Desktop", "Idle"):
            mode = display_config.get(mode_name, {})
            for date_key in ("LastSet", "LastUse"):
                if not isinstance(mode.get(date_key), datetime.datetime):
                    raise RuntimeError(f"macOS cannot decode {display}.{mode_name}: missing {date_key}")
            if not {"Choices", "Shuffle", "EncodedOptionValues"}.issubset(mode.get("Content", {})):
                raise RuntimeError(f"macOS cannot decode {display}.{mode_name}.Content")
        configuration = display_config["Idle"]["Content"]
        choices = configuration.get("Choices", [])
        if not choices or choices[0].get("Provider") != PROVIDER:
            raise RuntimeError(f"macOS is not using an Aerial wallpaper on display {display}")
        asset_id = plistlib.loads(choices[0]["Configuration"]).get("assetID", "").upper()
        if asset_id != slot.upper():
            raise RuntimeError(f"Display {display} points to {asset_id}, but Aura saved {slot}")
        if asset_id not in manifest_ids:
            raise RuntimeError(f"Aerial catalog does not contain slot {asset_id}")
        video = AERIALS / "videos" / f"{asset_id}.mov"
        signature = (AURA / f"{asset_id}.source").read_text().strip()
        converted = AURA / "Converted" / f"{signature}.mov"
        if not filecmp.cmp(video, converted, shallow=False):
            raise RuntimeError(f"macOS replaced Aura's video in {asset_id}; it no longer matches the conversion")
        if not video.is_file() or not temporal_groups(video).issuperset({"sgpd tscl", "sgpd tsas", "csgm tscl", "csgm tsas"}):
            raise RuntimeError(f"Slot {asset_id} is missing its playable HEVC temporal video")
        print(f"OK {display} -> {asset_id} ({video.stat().st_size // (1024 * 1024)} MiB)")

    subprocess.run(["pgrep", "-x", "aura"], check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["pgrep", "-x", "aura-bg"], check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["pgrep", "-x", "WallpaperAgent"], check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["pgrep", "-x", "WallpaperAerialsExtension"], check=True, stdout=subprocess.DEVNULL)
    print("OK Aura, desktop player and macOS Aerial player are running; visual lock/unlock check still required")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
