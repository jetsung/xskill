#!/usr/bin/env python3
"""
Collect skills from configured source repositories and generate skills.json.

The output format matches the CacheData struct used by xskill:
{
  "updated_at": "ISO8601",
  "sources": [
    {
      "source": "org/repo",
      "url": "https://...",
      "skills": [
        {"name": "...", "path": "skills/.../SKILL.md", "description": "...", "version": "..."}
      ]
    }
  ]
}

Usage:
  SOURCES='[{"url":"https://github.com/antfu/skills"}]' python collect_skills.py
"""

import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse


def derive_name(url: str) -> str:
    """Derive a short source name from a Git URL.

    Extracts the last two path segments (org/repo), stripping .git suffix.
    Falls back to the full URL on parse failure.
    """
    try:
        parsed = urlparse(url)
        parts = [p for p in parsed.path.strip("/").split("/") if p]
        if len(parts) >= 2:
            name = "/".join(parts[-2:])
            return name.removesuffix(".git")
    except Exception:
        pass
    return url


def parse_frontmatter(content: str) -> dict:
    """Parse YAML frontmatter from SKILL.md content.

    Handles simple flat key-value pairs and one level of nesting
    (e.g. metadata.version). Does not require pyyaml.
    """
    match = re.match(r"^---\s*\n(.*?)\n---", content, re.DOTALL)
    if not match:
        return {}

    fm = match.group(1)
    result = {}
    current_key = None

    for line in fm.split("\n"):
        line = line.rstrip()
        if not line or line.lstrip().startswith("#"):
            continue

        # Top-level key (no leading whitespace)
        if not line.startswith(" ") and ":" in line:
            key, _, value = line.partition(":")
            key = key.strip()
            value = value.strip().strip("\"'")
            if value:
                result[key] = value
                current_key = None
            else:
                current_key = key
            continue

        # Nested key under current_key (2-space indent)
        if current_key and line.startswith("  ") and not line.startswith("    "):
            m = re.match(r"^\s+(\w[\w_-]*):\s*(.*)", line)
            if m:
                sub_key, value = m.groups()
                value = value.strip().strip("\"'")
                if value:
                    result[f"{current_key}.{sub_key}"] = value

    return result


def clone_repo(url: str, dest: str) -> bool:
    """Shallow-clone a git repository."""
    try:
        subprocess.run(
            ["git", "clone", "--depth=1", "--quiet", url, dest],
            capture_output=True,
            timeout=180,
            check=True,
        )
        return True
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
        print(f"  WARNING: clone failed: {e}", file=sys.stderr)
        return False


def collect_from_repo(repo_url: str) -> list[dict]:
    """Clone a repo and collect all skills from its skills/ directory."""
    skills = []

    with tempfile.TemporaryDirectory(prefix="xskill-") as tmpdir:
        if not clone_repo(repo_url, tmpdir):
            return skills

        skills_dir = Path(tmpdir) / "skills"
        if not skills_dir.exists():
            print(f"  No skills/ directory, skipping")
            return skills

        for skill_md in sorted(skills_dir.rglob("SKILL.md")):
            rel_path = skill_md.relative_to(tmpdir)
            skill_dir = skill_md.parent

            try:
                content = skill_md.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                content = skill_md.read_text(encoding="latin-1")

            meta = parse_frontmatter(content)

            name = meta.get("name") or skill_dir.name
            description = meta.get("description", "")
            version = meta.get("metadata.version", "")

            skills.append(
                {
                    "name": name,
                    "path": str(rel_path),
                    "description": description,
                    "version": version,
                }
            )

    return skills


def main() -> None:
    sources_raw = os.environ.get("SOURCES", "[]")
    try:
        sources = json.loads(sources_raw)
    except json.JSONDecodeError as e:
        print(f"ERROR: Failed to parse SOURCES env var: {e}", file=sys.stderr)
        sys.exit(1)

    if not sources:
        print("No sources configured, generating empty index")

    now = datetime.now(timezone.utc)
    timestamp = now.strftime("%Y-%m-%dT%H:%M:%S.") + f"{now.microsecond // 1000:03d}Z"

    result_sources = []
    total_skills = 0

    for src in sources:
        url = src["url"]
        name = src.get("name", "")
        print(f"::group::Collecting from {name}")
        print(f"  URL: {url}")

        skills = collect_from_repo(url)
        print(f"  Found {len(skills)} skills")
        total_skills += len(skills)

        result_sources.append({"source": name, "url": url, "skills": skills})
        print("::endgroup::")

    data = {
        "$schema": "https://xskill.gcli.cn/registry.schema.json",
        "updated_at": timestamp,
        "sources": result_sources,
    }

    output_dir = Path("output")
    output_dir.mkdir(exist_ok=True)
    output_file = output_dir / "skills.json"

    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)

    print(f"\nGenerated {output_file}: {total_skills} skills from {len(result_sources)} sources")


if __name__ == "__main__":
    main()
