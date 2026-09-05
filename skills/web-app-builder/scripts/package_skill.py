#!/usr/bin/env python3
"""Package web-app-builder skill into distributable .skill file."""

import json
import os
import shutil
import sys
import zipfile

def get_skill_info(skill_path):
    """Read skill metadata."""
    meta_path = os.path.join(skill_path, "_meta.json")
    with open(meta_path, 'r', encoding='utf-8') as f:
        return json.load(f)

def create_package(skill_path, output_dir):
    """Create .skill package."""
    skill_name = os.path.basename(skill_path)
    meta = get_skill_info(skill_path)

    output_file = os.path.join(output_dir, f"{skill_name}.skill")

    # Create zip archive
    with zipfile.ZipFile(output_file, 'w', zipfile.ZIP_DEFLATED) as zf:
        for root, dirs, files in os.walk(skill_path):
            # Skip hidden files and directories
            dirs[:] = [d for d in dirs if not d.startswith('.')]
            files = [f for f in files if not f.startswith('.') and not f.endswith('.pyc')]

            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, os.path.dirname(skill_path))
                zf.write(file_path, arcname)

    print(f"Package created: {output_file}")
    print(f"Skill ID: {meta['id']}")
    print(f"Version: {meta['version']}")
    return output_file

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    skill_path = os.path.dirname(script_dir)

    if len(sys.argv) > 1:
        skill_path = sys.argv[1]

    output_dir = os.path.join(script_dir, '..')

    print(f"Packaging skill from: {skill_path}")
    print("-" * 50)

    output_file = create_package(skill_path, output_dir)

    print("-" * 50)
    print(f"Done! Package: {output_file}")

if __name__ == "__main__":
    main()
