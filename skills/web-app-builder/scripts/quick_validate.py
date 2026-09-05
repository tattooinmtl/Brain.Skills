#!/usr/bin/env python3
"""Validate web-app-builder skill structure and frontmatter."""

import json
import os
import re
import sys

def validate_frontmatter(skill_path):
    """Validate SKILL.md frontmatter."""
    skill_md_path = os.path.join(skill_path, "SKILL.md")

    if not os.path.exists(skill_md_path):
        return False, "SKILL.md not found"

    with open(skill_md_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Check if file starts with frontmatter
    if not content.startswith('---'):
        return False, "SKILL.md must start with frontmatter (---)"

    # Extract frontmatter
    parts = content.split('---', 2)
    if len(parts) < 3:
        return False, "Invalid frontmatter format"

    frontmatter = parts[1]
    body = parts[2]

    # Check required fields
    if 'name:' not in frontmatter:
        return False, "Frontmatter missing 'name' field"

    if 'description:' not in frontmatter:
        return False, "Frontmatter missing 'description' field"

    # Validate name matches folder
    name_match = re.search(r'^name:\s*(.+)$', frontmatter, re.MULTILINE)
    if name_match:
        skill_name = name_match.group(1).strip()
        folder_name = os.path.basename(skill_path)
        if skill_name != folder_name:
            return False, f"Name mismatch: frontmatter says '{skill_name}', folder is '{folder_name}'"

    # Check frontmatter is properly closed and content exists after
    if not body.strip():
        return False, "SKILL.md has no content after frontmatter"

    # Check frontmatter doesn't have content after it (should be only 2 --- markers)
    if parts[2].startswith('---'):
        return False, "Frontmatter not properly closed"

    return True, "Frontmatter validation passed"

def validate_meta(skill_path):
    """Validate _meta.json."""
    meta_path = os.path.join(skill_path, "_meta.json")

    if not os.path.exists(meta_path):
        return False, "_meta.json not found"

    try:
        with open(meta_path, 'r', encoding='utf-8') as f:
            meta = json.load(f)

        if 'id' not in meta:
            return False, "_meta.json missing 'id' field"

        if 'version' not in meta:
            return False, "_meta.json missing 'version' field"

        # Validate semver format
        version = meta['version']
        if not re.match(r'^\d+\.\d+\.\d+$', version):
            return False, f"Invalid semver format: {version}"

        return True, "_meta.json validation passed"
    except json.JSONDecodeError as e:
        return False, f"Invalid JSON in _meta.json: {e}"

def main():
    skill_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    print(f"Validating skill at: {skill_path}")
    print("-" * 50)

    all_passed = True

    # Validate frontmatter
    passed, msg = validate_frontmatter(skill_path)
    print(f"[{'PASS' if passed else 'FAIL'}] {msg}")
    all_passed = all_passed and passed

    # Validate meta
    passed, msg = validate_meta(skill_path)
    print(f"[{'PASS' if passed else 'FAIL'}] {msg}")
    all_passed = all_passed and passed

    print("-" * 50)
    if all_passed:
        print("All validations passed!")
        sys.exit(0)
    else:
        print("Validation failed!")
        sys.exit(1)

if __name__ == "__main__":
    main()
