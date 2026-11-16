#!/usr/bin/env python3
"""F2 mechanical migration: crates/ → Sprawling/ + Tools/.

Phases:
  rename  — git mv package directories (pure rename commit material)
  rewrite — table-driven path text rewrite across the tree
  table   — mark product roots current and retire legacy crates/

Run from the repository root. Does not commit.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# leaf under crates/ → (product_root, destination leaf)
# sprawling assembly package → Sprawling/app
PRODUCT = {
    "accounting": ("Sprawling", "accounting"),
    "agent_protocols": ("Sprawling", "agent_protocols"),
    "browser": ("Sprawling", "browser"),
    "child": ("Sprawling", "child"),
    "city": ("Sprawling", "city"),
    "collab": ("Sprawling", "collab"),
    "desktop": ("Sprawling", "desktop"),
    "documents": ("Sprawling", "documents"),
    "gateway": ("Sprawling", "gateway"),
    "kernel": ("Sprawling", "kernel"),
    "remote_access": ("Sprawling", "remote_access"),
    "runtime": ("Sprawling", "runtime"),
    "sprawling": ("Sprawling", "app"),
    "storage": ("Sprawling", "storage"),
    "wire": ("Sprawling", "wire"),
}

TOOLS = {
    "adversary": ("Tools", "adversary"),
    "benchstat": ("Tools", "benchstat"),
    "citysim": ("Tools", "citysim"),
    "fixtures": ("Tools", "fixtures"),
    "fuzz": ("Tools", "fuzz"),
    "xtask": ("Tools", "xtask"),
}

SKIP_DIR_NAMES = {
    ".git",
    "target",
    "node_modules",
    "local",
    "web-dist",
    ".lake",
    "dist",
}

TEXT_SUFFIXES = {
    "",
    ".rs",
    ".toml",
    ".lean",
    ".md",
    ".yml",
    ".yaml",
    ".json",
    ".jsonl",
    ".nix",
    ".sh",
    ".ps1",
    ".just",
    ".ts",
    ".tsx",
    ".js",
    ".mjs",
    ".cjs",
    ".svelte",
    ".css",
    ".html",
    ".txt",
    ".tsv",
    ".csv",
    ".py",
    ".zig",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".svg",
}


def run(cmd: list[str]) -> None:
    print("+", " ".join(cmd), flush=True)
    subprocess.check_call(cmd, cwd=ROOT)


def slash_pairs() -> list[tuple[str, str]]:
    """Longest-first old→new slash path pairs under crates/."""
    pairs: list[tuple[str, str]] = []
    # desktop/ffi rides with desktop directory move; still need string map
    pairs.append(("crates/desktop/ffi", "Sprawling/desktop/ffi"))
    for leaf, (root, dest) in {**PRODUCT, **TOOLS}.items():
        pairs.append((f"crates/{leaf}", f"{root}/{dest}"))
    pairs.sort(key=lambda p: len(p[0]), reverse=True)
    return pairs


def dotted_pairs() -> list[tuple[str, str]]:
    """Lean / modmap dotted module prefixes."""
    pairs: list[tuple[str, str]] = []
    pairs.append(("crates.desktop.ffi", "Sprawling.desktop.ffi"))
    for leaf, (root, dest) in {**PRODUCT, **TOOLS}.items():
        old = f"crates.{leaf}"
        new = f"{root}.{dest}"
        pairs.append((old, new))
    pairs.sort(key=lambda p: len(p[0]), reverse=True)
    return pairs


def rename() -> None:
    for name in ("Sprawling", "Tools"):
        path = ROOT / name
        path.mkdir(exist_ok=True)
        keep = path / ".gitkeep"
        if not any(path.iterdir()):
            keep.write_text("")
    # Move whole desktop (includes ffi) before other leaves.
    order = [
        "desktop",  # includes ffi
        "sprawling",
        "accounting",
        "agent_protocols",
        "browser",
        "child",
        "city",
        "collab",
        "documents",
        "gateway",
        "kernel",
        "remote_access",
        "runtime",
        "storage",
        "wire",
        "xtask",
        "citysim",
        "benchstat",
        "fixtures",
        "adversary",
        "fuzz",
    ]
    mapping = {**PRODUCT, **TOOLS}
    for leaf in order:
        root_name, dest = mapping[leaf]
        src = ROOT / "crates" / leaf
        dst = ROOT / root_name / dest
        if not src.exists():
            raise SystemExit(f"missing source {src}")
        if dst.exists():
            raise SystemExit(f"destination exists {dst}")
        dst.parent.mkdir(parents=True, exist_ok=True)
        run(["git", "mv", str(src.relative_to(ROOT)), str(dst.relative_to(ROOT))])

    # Remaining tracked files under crates/ (README, development-tools.md).
    leftover = sorted(
        p for p in (ROOT / "crates").rglob("*") if p.is_file() and ".git" not in p.parts
    )
    for path in leftover:
        rel = path.relative_to(ROOT)
        if path.name == "README.md":
            # Content is rewritten later; park under Sprawling for the rename commit.
            run(["git", "mv", str(rel), "Sprawling/README.md"])
        elif path.name == "development-tools.md":
            run(["git", "mv", str(rel), "Tools/development-tools.md"])
        else:
            raise SystemExit(f"unmapped leftover under crates/: {rel}")

    crates_dir = ROOT / "crates"
    if any(crates_dir.iterdir()):
        raise SystemExit(f"crates/ not empty after moves: {list(crates_dir.iterdir())}")
    crates_dir.rmdir()
    # Drop placeholder keep files if present.
    for name in ("Sprawling", "Tools"):
        keep = ROOT / name / ".gitkeep"
        if keep.exists() and keep.stat().st_size == 0:
            keep.unlink()


def should_skip(path: Path) -> bool:
    parts = set(path.parts)
    if parts & SKIP_DIR_NAMES:
        return True
    if path.is_symlink():
        return True
    if not path.is_file():
        return True
    name = path.name
    suffix = path.suffix.lower()
    if name in {"Cargo.lock", "justfile", "Justfile", "lakefile.toml", "AGENTS.md"}:
        return False
    if name.startswith("Dockerfile"):
        return False
    if suffix in TEXT_SUFFIXES or name in TEXT_SUFFIXES:
        return False
    # extensionless text that we still want
    if suffix == "" and name in {
        "zig-version",
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "rust-toolchain",
    }:
        return False
    return True


def rewrite_text(text: str) -> str:
    for old, new in slash_pairs():
        text = text.replace(old, new)
        text = text.replace(old.replace("/", "\\"), new.replace("/", "\\"))
    for old, new in dotted_pairs():
        text = text.replace(old, new)
    return text


def rewrite_tree() -> int:
    changed = 0
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIR_NAMES]
        for name in filenames:
            path = Path(dirpath) / name
            if should_skip(path):
                continue
            try:
                raw = path.read_bytes()
            except OSError:
                continue
            if b"\0" in raw:
                continue
            try:
                text = raw.decode("utf-8")
            except UnicodeDecodeError:
                continue
            new = rewrite_text(text)
            if new != text:
                path.write_text(new, encoding="utf-8", newline="\n")
                changed += 1
                print(f"rewrite {path.relative_to(ROOT)}")
    return changed


def update_product_table() -> None:
    path = ROOT / "architecture.toml"
    text = path.read_text(encoding="utf-8")
    text = text.replace(
        'name = "Sprawling"\npath = "Sprawling"\nlayout = "planned"',
        'name = "Sprawling"\npath = "Sprawling"\nlayout = "current"',
        1,
    )
    text = text.replace(
        'name = "Tools"\npath = "Tools"\nlayout = "planned"',
        'name = "Tools"\npath = "Tools"\nlayout = "current"',
        1,
    )
    text = text.replace(
        'path = "crates"\nstatus = "live"',
        'path = "crates"\nstatus = "retired"',
        1,
    )
    path.write_text(text, encoding="utf-8", newline="\n")
    print("updated architecture.toml product_root / legacy_path")

    arch = ROOT / "docs" / "ARCHITECTURE.md"
    arch_text = arch.read_text(encoding="utf-8")
    old = (
        "During transition the table may list planned roots beside the paths\n"
        "  the tree still holds (`crates/`, `client/`)."
    )
    new = (
        "After the crates split, `Sprawling/` and `Tools/` are current and\n"
        "  `crates/` is retired; `client/` remains live until RefRain R0."
    )
    if old in arch_text:
        arch.write_text(arch_text.replace(old, new, 1), encoding="utf-8", newline="\n")
        print("updated docs/ARCHITECTURE.md transition prose")
    else:
        print("ARCHITECTURE transition prose already updated or missing")


def write_path_map() -> None:
    lines = [
        "# F2 path map — crates/ → Sprawling/ + Tools/",
        "",
        "Mechanical migration only. Package, lib, and online protocol names are unchanged.",
        "",
        "| Old path | New path | Kind |",
        "|---|---|---|",
    ]
    rows = []
    for leaf, (root, dest) in sorted(PRODUCT.items()):
        rows.append((f"crates/{leaf}", f"{root}/{dest}", "product"))
    rows.append(("crates/desktop/ffi", "Sprawling/desktop/ffi", "product"))
    for leaf, (root, dest) in sorted(TOOLS.items()):
        rows.append((f"crates/{leaf}", f"{root}/{dest}", "tool"))
    rows.append(("crates/README.md", "Sprawling/README.md (+ Tools/README.md)", "docs"))
    rows.append(("crates/development-tools.md", "Tools/development-tools.md", "docs"))
    for old, new, kind in rows:
        lines.append(f"| `{old}` | `{new}` | {kind} |")
    lines.append("")
    out = ROOT / "docs" / "f2-path-map.md"
    # Prefer internal store report rather than docs unless needed —
    # repository docs/ is for finished published docs. Keep map next to Tools.
    out = ROOT / "Tools" / "PATH-MAP.md"
    out.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {out.relative_to(ROOT)}")


def write_tools_readme() -> None:
    """Split development-tools into Tools/README.md; keep development-tools as alias prose."""
    src = ROOT / "Tools" / "development-tools.md"
    text = src.read_text(encoding="utf-8")
    text = rewrite_text(text)
    # Title and crates/ framing.
    text = text.replace("# Development tools\n", "# Tools\n\n", 1)
    text = text.replace(
        "These directories under `Tools/` hold what checks",
        "These directories under `Tools/` hold what checks",
    )
    # After slash rewrite, links already point at Tools/...
    readme = ROOT / "Tools" / "README.md"
    readme.write_text(text, encoding="utf-8", newline="\n")
    src.write_text(
        "# Development tools\n\n"
        "Moved to [`README.md`](./README.md). This file remains as a pointer "
        "for links that still name `development-tools.md`.\n",
        encoding="utf-8",
        newline="\n",
    )
    print("wrote Tools/README.md")


def write_sprawling_readme() -> None:
    path = ROOT / "Sprawling" / "README.md"
    text = path.read_text(encoding="utf-8")
    text = rewrite_text(text)
    text = text.replace("# crates\n", "# Sprawling\n", 1)
    text = text.replace(
        "Every Rust package of the city lives in this directory.",
        "Every product Rust package of the city lives under `Sprawling/`.",
        1,
    )
    text = text.replace(
        "Development-only packages and fixtures also live here; [Development tools](development-tools.md) lists them.",
        "Development-only packages and fixtures live under [`Tools/`](../Tools/README.md).",
        1,
    )
    text = text.replace("## The crates\n", "## The packages\n", 1)
    path.write_text(text, encoding="utf-8", newline="\n")
    print("updated Sprawling/README.md")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "phase",
        choices=["rename", "rewrite", "table", "readmes", "path-map", "all-after-rename"],
    )
    args = parser.parse_args()
    os.chdir(ROOT)
    if args.phase == "rename":
        rename()
    elif args.phase == "rewrite":
        n = rewrite_tree()
        print(f"rewrote {n} files")
    elif args.phase == "table":
        update_product_table()
    elif args.phase == "readmes":
        write_sprawling_readme()
        write_tools_readme()
    elif args.phase == "path-map":
        write_path_map()
    elif args.phase == "all-after-rename":
        n = rewrite_tree()
        print(f"rewrote {n} files")
        update_product_table()
        write_sprawling_readme()
        write_tools_readme()
        write_path_map()


if __name__ == "__main__":
    main()
