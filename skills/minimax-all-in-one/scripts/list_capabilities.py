#!/usr/bin/env python3
import json
from pathlib import Path

manifest_path = Path(__file__).resolve().parents[1] / 'assets' / 'source_manifest.json'
manifest = json.loads(manifest_path.read_text())
for item in manifest['skills']:
    print(f"{item['name']}: {item['description']}")
