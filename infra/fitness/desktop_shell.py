# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Fitness check for the native desktop shell (ADR-0044)."""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    problems: list[str] = []
    details: list[str] = []

    desktop = repo_root / "apps/desktop"
    manifest = desktop / "Cargo.toml"
    source = desktop / "src/main.rs"
    packaging = repo_root / "scripts/package/macos-app.sh"
    for path in (manifest, source, packaging):
        if not path.is_file():
            return CheckResult(name="desktop_shell", ok=False,
                               summary=f"missing {path.relative_to(repo_root)}", details=[])

    manifest_text = manifest.read_text()
    source_text = source.read_text()
    script_text = packaging.read_text()

    for crate in ("wry", "tao", "muda", "aether_web"):
        if crate not in manifest_text:
            problems.append(f"desktop manifest is missing {crate}")
    if "aether_cli" in manifest_text:
        problems.append("desktop shell must depend on aether_web, not aether_cli")
    if "127.0.0.1:0" not in source_text:
        problems.append("desktop shell must bind an ephemeral loopback port")
    if "generate_token" not in source_text:
        problems.append("desktop shell must generate a per-launch token")
    if "with_navigation_handler" not in source_text:
        problems.append("desktop WebView must restrict navigation")
    for marker in (
        "MenuItem::with_id", "Accelerator::new", "recent-projects.json",
        "window.json", "/api/project", "Code::KeyO",
        "with_decorations(false)", "with_ipc_handler",
    ):
        if marker not in source_text:
            problems.append(f"desktop shell is missing native affordance {marker}")
    for forbidden in ("Command::new(\"sh\")", "std::process::Command::new(\"bash\")"):
        if forbidden in source_text:
            problems.append(f"desktop shell must not spawn a shell: {forbidden}")

    for resource in ("contracts", "reference_plugins", "python", "examples"):
        if resource not in script_text:
            problems.append(f"bundle script does not include {resource}")
    if "fetch-python.sh" not in script_text:
        problems.append("bundle script does not install the Python runtime")
    if "codesign" not in script_text:
        problems.append("bundle script does not sign the application")
    if "AETHER_MONOREPO" not in source_text or "AETHER_PYTHON" not in source_text:
        problems.append("desktop shell does not configure bundled resource paths")

    # Optional bundle checks run only when an app has actually been built.
    app = repo_root / "dist/Aether.app"
    skipped = False
    if not app.is_dir():
        skipped = True
        details.append("no dist/Aether.app; source invariants only")
    else:
        contents = app / "Contents"
        plist = contents / "Info.plist"
        resources = contents / "Resources"
        if not plist.is_file():
            problems.append("bundle has no Info.plist")
        else:
            for key in ("CFBundleExecutable", "CFBundleIdentifier", "Aether"):
                if key not in plist.read_text():
                    problems.append(f"Info.plist is missing {key}")
        if not (contents / "MacOS/Aether").is_file():
            problems.append("bundle has no MacOS/Aether executable")
        for path in ("bin/aether", "aether/contracts/graph.model/catalog/nodes/requirement.node.json"):
            if not (resources / path).exists():
                problems.append(f"bundle is missing {path}")
        if (resources / "web.token").exists():
            problems.append("bundle contains a token file")
        details.append("bundle layout verified")

    if problems:
        return CheckResult(name="desktop_shell", ok=False,
                           summary=f"{len(problems)} desktop shell problem(s)",
                           details=problems + details)
    return CheckResult(
        name="desktop_shell", ok=True, skipped=skipped,
        summary="wry shell is CLI-bound and loopback-only"
                + ("; bundle absent" if skipped else "; bundle verified"),
        details=details + ["in-process server, ephemeral port, per-launch token",
                           "system WebView, no external navigation"],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok or result.skipped else 1)
