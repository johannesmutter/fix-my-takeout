#!/usr/bin/env python3
"""Post-release verification for updater health."""

from __future__ import annotations

import json
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[1]
TAURI_CONF_PATH = ROOT_DIR / "src-tauri" / "tauri.conf.json"
REPO = "johannesmutter/fix-my-takeout"


def run_command(command: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(
        command,
        cwd=str(cwd) if cwd else None,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        message = (result.stderr or result.stdout or "unknown error").strip()
        raise RuntimeError(f"{' '.join(command)} failed: {message}")
    return (result.stdout or "").strip()


def fetch_json(url: str) -> dict:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": "fix-my-takeout-release-verify/1.0",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        raise RuntimeError(f"HTTP {e.code} for {url}") from e
    except urllib.error.URLError as e:
        raise RuntimeError(f"Could not reach {url}: {e.reason}") from e


def print_check(title: str, ok: bool, detail: str) -> None:
    marker = "PASS" if ok else "FAIL"
    print(f"[{marker}] {title}: {detail}")


def get_configured_endpoint() -> str:
    with TAURI_CONF_PATH.open("r", encoding="utf-8") as fh:
        config = json.load(fh)
    endpoints = config.get("plugins", {}).get("updater", {}).get("endpoints", [])
    if not endpoints:
        raise RuntimeError("No updater endpoint configured in tauri.conf.json")
    return str(endpoints[0])


def verify_repo_visibility() -> tuple[bool, str]:
    try:
        output = run_command(
            ["gh", "repo", "view", REPO, "--json", "visibility,isPrivate"],
            cwd=ROOT_DIR,
        )
    except Exception as e:
        return False, f"Could not inspect repo visibility ({e})"
    payload = json.loads(output)
    visibility = payload.get("visibility", "unknown")
    is_private = bool(payload.get("isPrivate", False))
    if is_private:
        return False, f"repository is private ({visibility})"
    return True, f"repository visibility is {visibility.lower()}"


def verify_endpoint_fetch(endpoint_url: str) -> tuple[bool, str, dict | None]:
    try:
        payload = fetch_json(endpoint_url)
    except Exception as e:
        return False, str(e), None
    version = str(payload.get("version", "missing"))
    platforms = payload.get("platforms")
    if not isinstance(platforms, dict) or len(platforms) == 0:
        return False, "JSON has no platforms section", payload
    return True, f"version={version}, platforms={len(platforms)}", payload


def verify_release_assets() -> tuple[bool, str]:
    try:
        output = run_command(
            ["gh", "api", f"repos/{REPO}/releases/latest", "--jq", ".assets[].name"],
            cwd=ROOT_DIR,
        )
    except Exception as e:
        return False, f"Could not read latest release assets ({e})"

    names = [l.strip() for l in output.splitlines() if l.strip()]
    if "latest.json" not in names:
        return False, "latest release does not contain latest.json asset"

    has_tarball = any(n.endswith(".app.tar.gz") for n in names)
    has_sig = any(n.endswith(".app.tar.gz.sig") for n in names)
    if not has_tarball or not has_sig:
        return False, "missing macOS updater tarball/signature assets"

    return True, f"{len(names)} assets present including updater artifacts"


def main() -> int:
    try:
        endpoint_url = get_configured_endpoint()

        print("Release verification")
        print(f"Updater endpoint: {endpoint_url}")
        print()

        repo_ok, repo_detail = verify_repo_visibility()
        print_check("Repository visibility", repo_ok, repo_detail)

        assets_ok, assets_detail = verify_release_assets()
        print_check("Latest release assets", assets_ok, assets_detail)

        endpoint_ok, endpoint_detail, endpoint_payload = verify_endpoint_fetch(endpoint_url)
        print_check("Updater endpoint fetch", endpoint_ok, endpoint_detail)

        version_match_ok = False
        if endpoint_payload:
            remote_version = str(endpoint_payload.get("version", "missing"))
            with TAURI_CONF_PATH.open("r", encoding="utf-8") as fh:
                local_version = json.load(fh)["version"]
            version_match_ok = remote_version == local_version
            detail = f"endpoint={remote_version}, local={local_version}"
            print_check("Version alignment", version_match_ok, detail)

        all_ok = repo_ok and assets_ok and endpoint_ok and version_match_ok
        print()
        if all_ok:
            print("All checks passed.")
            return 0
        print("Some checks failed. Review before shipping updates.")
        return 1
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
