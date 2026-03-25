#!/usr/bin/env python3
"""Interactive release wizard for Fix My Takeout.

Automates:
- synchronized version bump across Tauri, Cargo, npm, and README
- optional commit / push / tag / tag push flow
- preflight safety checks to avoid mismatched release tags
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


VERSION_PATTERN = re.compile(r"^\d+\.\d+\.\d+$")
ROOT_DIR = Path(__file__).resolve().parents[1]
TAURI_CONF_PATH = ROOT_DIR / "src-tauri" / "tauri.conf.json"
CARGO_TOML_PATH = ROOT_DIR / "src-tauri" / "Cargo.toml"
PACKAGE_JSON_PATH = ROOT_DIR / "package.json"
README_PATH = ROOT_DIR / "README.md"

REPO = "johannesmutter/fix-my-takeout"
PRODUCT_NAME = "Fix My Takeout"
DMG_BASE = "Fix.My.Takeout"


@dataclass
class VersionState:
    tauri_conf_version: str
    cargo_version: str
    package_version: str

    def is_synchronized(self) -> bool:
        return (
            self.tauri_conf_version == self.cargo_version
            and self.cargo_version == self.package_version
        )

    def canonical(self) -> str:
        return self.cargo_version


def run_command(command: list[str], cwd: Path | None = None, capture: bool = False) -> str:
    result = subprocess.run(
        command,
        cwd=str(cwd) if cwd else None,
        text=True,
        capture_output=capture,
        check=False,
    )
    if result.returncode != 0:
        stderr_text = (result.stderr or "").strip()
        stdout_text = (result.stdout or "").strip()
        message = stderr_text or stdout_text or "unknown command error"
        raise RuntimeError(f"{' '.join(command)} failed: {message}")
    return (result.stdout or "").strip()


def prompt_yes_no(prompt_text: str, default_yes: bool = True) -> bool:
    suffix = "[Y/n]" if default_yes else "[y/N]"
    while True:
        answer = input(f"{prompt_text} {suffix}: ").strip().lower()
        if answer == "":
            return default_yes
        if answer in {"y", "yes"}:
            return True
        if answer in {"n", "no"}:
            return False
        print("Please answer with 'y' or 'n'.")


def parse_semver(version: str) -> tuple[int, int, int]:
    if not VERSION_PATTERN.match(version):
        raise ValueError(f"Invalid semantic version: {version}")
    major, minor, patch = version.split(".")
    return int(major), int(minor), int(patch)


def increment_version(version: str, mode: str) -> str:
    major, minor, patch = parse_semver(version)
    if mode == "patch":
        return f"{major}.{minor}.{patch + 1}"
    if mode == "minor":
        return f"{major}.{minor + 1}.0"
    if mode == "major":
        return f"{major + 1}.0.0"
    raise ValueError(f"Unsupported increment mode: {mode}")


def read_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def write_json(path: Path, content: dict) -> None:
    with path.open("w", encoding="utf-8") as fh:
        json.dump(content, fh, indent=2)
        fh.write("\n")


def read_cargo_version(path: Path) -> str:
    in_package = False
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            stripped = line.strip()
            if stripped.startswith("[") and stripped.endswith("]"):
                in_package = stripped == "[package]"
                continue
            if in_package and stripped.startswith("version"):
                match = re.match(r'version\s*=\s*"([^"]+)"', stripped)
                if match:
                    return match.group(1)
    raise RuntimeError(f"Could not find [package].version in {path}")


def write_cargo_version(path: Path, target_version: str) -> None:
    lines = path.read_text(encoding="utf-8").splitlines()
    in_package = False
    replaced = False
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_package = stripped == "[package]"
            continue
        if in_package and re.match(r'^\s*version\s*=\s*"[^"]+"\s*$', line):
            lines[i] = re.sub(r'"[^"]+"', f'"{target_version}"', line, count=1)
            replaced = True
            break
    if not replaced:
        raise RuntimeError(f"Could not replace [package].version in {path}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def update_readme_downloads(target_version: str) -> None:
    """Update download links in README.md to point to the new version."""
    text = README_PATH.read_text(encoding="utf-8")

    # Update direct download links: match the table rows with DMG links
    text = re.sub(
        r"\[Fix My Takeout [\d.]+ \(arm64\)\]\(https://github\.com/[^)]+\.dmg\)",
        f"[Fix My Takeout {target_version} (arm64)]"
        f"(https://github.com/{REPO}/releases/download/v{target_version}/{DMG_BASE}_{target_version}_aarch64.dmg)",
        text,
    )
    text = re.sub(
        r"\[Fix My Takeout [\d.]+ \(x64\)\]\(https://github\.com/[^)]+\.dmg\)",
        f"[Fix My Takeout {target_version} (x64)]"
        f"(https://github.com/{REPO}/releases/download/v{target_version}/{DMG_BASE}_{target_version}_x64.dmg)",
        text,
    )

    README_PATH.write_text(text, encoding="utf-8")


def read_version_state() -> VersionState:
    tauri_data = read_json(TAURI_CONF_PATH)
    pkg_data = read_json(PACKAGE_JSON_PATH)
    return VersionState(
        tauri_conf_version=str(tauri_data["version"]),
        cargo_version=read_cargo_version(CARGO_TOML_PATH),
        package_version=str(pkg_data["version"]),
    )


def ensure_required_tools() -> None:
    for cmd in ("git", "npm"):
        try:
            run_command([cmd, "--version"], capture=True)
        except RuntimeError as e:
            raise RuntimeError(f"Required command '{cmd}' is not available") from e


def update_versions(target_version: str) -> None:
    if not VERSION_PATTERN.match(target_version):
        raise ValueError(f"Invalid semantic version: {target_version}")

    tauri_data = read_json(TAURI_CONF_PATH)
    tauri_data["version"] = target_version
    write_json(TAURI_CONF_PATH, tauri_data)

    write_cargo_version(CARGO_TOML_PATH, target_version)

    pkg_data = read_json(PACKAGE_JSON_PATH)
    pkg_data["version"] = target_version
    write_json(PACKAGE_JSON_PATH, pkg_data)

    update_readme_downloads(target_version)


def select_target_version(base_version: str) -> str:
    patch = increment_version(base_version, "patch")
    minor = increment_version(base_version, "minor")
    major = increment_version(base_version, "major")

    print("\nChoose release version:")
    print(f"  1) Patch  -> {patch}")
    print(f"  2) Minor  -> {minor}")
    print(f"  3) Major  -> {major}")
    print("  4) Custom")
    print(f"  5) Keep current ({base_version})")

    while True:
        choice = input("Selection [1-5]: ").strip()
        if choice == "1":
            return patch
        if choice == "2":
            return minor
        if choice == "3":
            return major
        if choice == "4":
            custom = input("Enter custom semantic version (X.Y.Z): ").strip()
            if VERSION_PATTERN.match(custom):
                return custom
            print("Invalid semantic version format.")
            continue
        if choice == "5":
            return base_version
        print("Please choose a number between 1 and 5.")


def print_version_state(state: VersionState) -> None:
    print("\nCurrent version metadata:")
    print(f"  - tauri.conf.json: {state.tauri_conf_version}")
    print(f"  - Cargo.toml:      {state.cargo_version}")
    print(f"  - package.json:    {state.package_version}")
    print(f"  - synchronized:    {'yes' if state.is_synchronized() else 'no'}")


def guard_clean_worktree() -> None:
    status = run_command(["git", "status", "--porcelain"], cwd=ROOT_DIR, capture=True)
    if status:
        print("\nDetected local changes in the worktree.")
        print("Release tagging is safer from a clean state.")
        if not prompt_yes_no("Continue anyway?", default_yes=False):
            raise RuntimeError("Aborted due to dirty worktree.")


def create_release_commit(target_version: str) -> None:
    run_command(
        [
            "git", "add",
            str(TAURI_CONF_PATH.relative_to(ROOT_DIR)),
            str(CARGO_TOML_PATH.relative_to(ROOT_DIR)),
            str(PACKAGE_JSON_PATH.relative_to(ROOT_DIR)),
            str(README_PATH.relative_to(ROOT_DIR)),
            "src-tauri/Cargo.lock",
        ],
        cwd=ROOT_DIR,
    )
    run_command(["git", "commit", "-m", f"bump version to {target_version}"], cwd=ROOT_DIR)


def push_branch() -> None:
    branch = run_command(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=ROOT_DIR, capture=True
    )
    run_command(["git", "push", "origin", branch], cwd=ROOT_DIR)


def create_and_push_tag(target_version: str) -> None:
    tag = f"v{target_version}"
    existing = run_command(["git", "tag", "-l", tag], cwd=ROOT_DIR, capture=True)
    if existing.strip():
        raise RuntimeError(f"Tag {tag} already exists locally.")
    run_command(["git", "tag", tag], cwd=ROOT_DIR)
    run_command(["git", "push", "origin", tag], cwd=ROOT_DIR)


def print_next_steps(target_version: str) -> None:
    print("\nRelease wizard completed.")
    print("Next steps:")
    print(f"  1) Confirm Actions workflow ran for tag v{target_version}.")
    print("  2) Publish the draft GitHub release when all jobs pass.")
    print("  3) Validate updater endpoint:")
    print(f"     https://github.com/{REPO}/releases/latest/download/latest.json")
    print("  4) Run scripts/release_verify.py for post-release verification.")


def main() -> int:
    try:
        ensure_required_tools()
        guard_clean_worktree()

        current = read_version_state()
        print_version_state(current)

        base = current.canonical()
        if not current.is_synchronized():
            print("\nVersion files are not synchronized.")
            if not prompt_yes_no(
                "Use Cargo.toml as canonical base version and continue?",
                default_yes=True,
            ):
                print("No files changed.")
                return 0

        target = select_target_version(base)
        print(f"\nTarget version: {target}")
        if not prompt_yes_no("Apply version changes now?", default_yes=True):
            print("No files changed.")
            return 0

        update_versions(target)
        updated = read_version_state()
        print_version_state(updated)

        if not updated.is_synchronized() or updated.canonical() != target:
            raise RuntimeError("Version update failed consistency checks.")

        if prompt_yes_no("Create release commit now?", default_yes=True):
            create_release_commit(target)
            print("Release commit created.")

            if prompt_yes_no("Push branch to origin?", default_yes=True):
                push_branch()
                print("Branch pushed.")

            if prompt_yes_no(f"Create and push tag v{target}?", default_yes=True):
                create_and_push_tag(target)
                print(f"Tag v{target} pushed.")

        print_next_steps(target)
        return 0
    except KeyboardInterrupt:
        print("\nAborted by user.")
        return 130
    except Exception as e:  # pylint: disable=broad-exception-caught
        print(f"\nError: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
