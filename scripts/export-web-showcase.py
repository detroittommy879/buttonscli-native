"""Export ONLY personal theme documents, bundles and saved shaders for the web.

No credentials, provider settings, commands, paths or other preferences are
exported. Review the output before publishing personal theme metadata/code.
"""
import argparse
import json
from pathlib import Path


def export(profile, output):
    settings = json.loads((profile / "native.json").read_text(encoding="utf-8"))["preferences"]
    themes = []
    for path in sorted((profile / "themes").glob("*.json")):
        themes.append({"id": f"personal:{profile.name}:{path.stem}",
                       "document": json.loads(path.read_text(encoding="utf-8"))})
    payload = {"themes": themes, "bundles": settings.get("theme_bundles", []), "shaders": settings.get("saved_shaders", [])}
    output.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Exported {len(themes)} themes, {len(payload['bundles'])} bundles, {len(payload['shaders'])} shaders")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    export(args.profile, args.output)
