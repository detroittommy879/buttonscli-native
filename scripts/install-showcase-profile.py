"""Install a generated Motion Rooms pack into a closed native profile.

App closure and a compare-before-replace protect live settings. A full backup
stays beside native.json; existing theme files are never overwritten.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid


def install(pack_dir, profile_dir):
    profile_dir = profile_dir.resolve(strict=True)
    profile = profile_dir.name
    processes = subprocess.check_output([
        "powershell", "-NoProfile", "-Command",
        "@(Get-Process buttonscli -ErrorAction SilentlyContinue).Count"
    ], text=True).strip()
    if processes != "0":
        raise SystemExit("Close ButtonsCLI before installing profile bundles.")
    path = profile_dir / "native.json"
    original = path.read_bytes()
    document = json.loads(original)
    pack = json.loads((pack_dir / "pack.json").read_text(encoding="utf-8"))
    assert all(t["id"].startswith(f"personal:{profile}:") for t in pack["themes"])
    themes_dir = profile_dir / "themes"
    themes_dir.mkdir(exist_ok=True)
    # Check every collision before writing any asset.
    for theme in pack["themes"]:
        destination = themes_dir / theme["file"]
        if destination.exists() and json.loads(destination.read_text(encoding="utf-8")) != theme["document"]:
            raise SystemExit(f"Refusing to overwrite existing theme: {destination.name}")
    prefs = document["preferences"]
    for field, values in [("theme_bundles", pack["bundles"]), ("saved_shaders", pack["shaders"])]:
        existing = prefs.setdefault(field, [])
        names = {item["name"] for item in existing}
        if any(value["name"] in names and value not in existing for value in values):
            raise SystemExit(f"Existing {field} name conflicts; choose new names.")
        existing.extend(value for value in values if value["name"] not in names)
    if len(prefs["theme_bundles"]) > 64 or len(prefs["saved_shaders"]) > 64:
        raise SystemExit("Profile library capacity exceeded")
    backup = path.with_name("native-before-motion-rooms-" + uuid.uuid4().hex + ".json")
    shutil.copy2(path, backup)
    for theme in pack["themes"]:
        destination = themes_dir / theme["file"]
        if not destination.exists():
            shutil.copy2(pack_dir / theme["file"], destination)
    document["revision"] += 1
    staged = path.with_name("native-motion-rooms-" + uuid.uuid4().hex + ".tmp")
    staged.write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    if hashlib.sha256(path.read_bytes()).digest() != hashlib.sha256(original).digest():
        staged.unlink()
        raise SystemExit("Profile changed during install; backup retained, settings untouched.")
    os.replace(staged, path)
    print(f"Installed 6 bundles, 30 themes and 6 shaders into {profile}")
    print(f"Settings backup: {backup.name}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack_dir", type=Path)
    parser.add_argument("profile_dir", type=Path)
    args = parser.parse_args()
    install(args.pack_dir, args.profile_dir)
