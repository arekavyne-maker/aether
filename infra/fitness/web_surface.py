# SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
# SPDX-FileCopyrightText: 2026 Crocell Company
"""Fitness check for the local web interface (ADR-0042 / ADR-0043).

Enforces three invariants mechanically:

1. GUI/CLI parity (I-10): every discovered command maps to a workspace and
   every curated command still exists in the canonical CLI.
2. Air gap: the embedded console assets reference no external origin and
   the CSP contains no inline escape hatches.
3. No parallel implementation: the web crate must not depend on aether_cli.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def _cli_binary(root: Path) -> Path | None:
    for relative in ("target/debug/aether", "target/release/aether"):
        candidate = root / relative
        if candidate.is_file():
            return candidate
    return None


def check(repo_root: Path = REPO_ROOT):
    from run import CheckResult

    problems: list[str] = []
    details: list[str] = []

    catalog_path = repo_root / "crates/aether_web/catalog.json"
    try:
        catalog = json.loads(catalog_path.read_text())
    except Exception as exc:  # pragma: no cover - corrupt catalog is a hard fail
        return CheckResult(name="web_surface", ok=False,
                           summary=f"catalog unreadable: {exc}", details=[])

    workspaces = {w["id"] for w in catalog.get("workspaces", [])}
    families = catalog.get("families", {})
    curated = {c["name"] for c in catalog.get("commands", [])}

    if len(workspaces) != 10:
        problems.append(f"expected 10 workspaces, found {len(workspaces)}")
    if len(families) < 50:
        problems.append(f"expected at least 50 family mappings, found {len(families)}")
    for family, workspace in families.items():
        if workspace not in workspaces:
            problems.append(f"family {family} targets unknown workspace {workspace}")
    for command in catalog.get("commands", []):
        if command["workspace"] not in workspaces:
            problems.append(f"curated command {command['name']} has unknown workspace")

    # --- air gap: static assets must not reach outside the binary ---
    app = repo_root / "site/app"
    for path in sorted(app.rglob("*")):
        if not path.is_file() or path.suffix in (".mjs", ".woff2"):
            continue
        text = path.read_text(errors="ignore")
        for token in ("http://", "https://", "@import", "src=\"//"):
            if token in text:
                problems.append(f"{path.relative_to(repo_root)} contains external reference {token!r}")
        if path.suffix in (".js", ".html") and "style=" in text:
            problems.append(f"{path.relative_to(repo_root)} contains an inline style attribute")

    # --- dynamic console surface (ADR-0044) ---
    api_source = (repo_root / "crates/aether_web/src/api.rs").read_text()
    for route in (
        "/atlas", "/atlas/graph", "/atlas/schema", "/atlas/health",
        "/project", "/jobs", "/jobs/:id", "/jobs/:id/events", "/jobs/:id/cancel",
    ):
        if f'route("{route}"' not in api_source:
            problems.append(f"missing API route {route}")
    atlas_source = (repo_root / "crates/aether_web/src/atlas.rs").read_text()
    if "MAX_NODES" not in atlas_source or "MAX_EDGES" not in atlas_source:
        problems.append("atlas has no truncation caps")
    for module in (
        "viz/graph.js", "viz/layout.js", "data/atlas.js", "views/atlas.js",
        "jobs.js", "state/router.js",
    ):
        if not (app / "js" / module).is_file():
            problems.append(f"missing console module {module}")
    jobs_source = (app / "js" / "jobs.js").read_text()
    if "events?after=" not in jobs_source or "cancel" not in jobs_source:
        problems.append("live jobs lack an event cursor or cancellation")

    assets = (repo_root / "crates/aether_web/src/assets.rs").read_text()
    csp_lines = [line for line in assets.splitlines() if "-src " in line]
    csp = " ".join(csp_lines)
    if "default-src 'self'" not in csp or "script-src 'self'" not in csp:
        problems.append("CSP does not pin default-src and script-src to 'self'")
    for forbidden in ("unsafe-inline", "unsafe-eval"):
        if forbidden in csp:
            problems.append(f"CSP allows {forbidden}")
    if '"jobs.js"' not in assets:
        problems.append("embedded assets do not include jobs.js")
    if 'route("/assets/fonts/*file"' not in api_source or "FONT_ASSETS" not in assets:
        problems.append("local fonts are not embedded and served from the same origin")
    for font in ("urbanist-latin.woff2", "arapey-italic-latin.woff2"):
        if not (app / "fonts" / font).is_file():
            problems.append(f"missing bundled font {font}")
    index = (app / "index.html").read_text()
    if "/assets/js/main.js" not in index or "/assets/app.css" not in index:
        problems.append("index.html does not reference the embedded local assets")

    # --- no parallel implementation behind the GUI ---
    manifest = (repo_root / "crates/aether_web/Cargo.toml").read_text()
    if "aether_cli" in manifest:
        problems.append("aether_web must not depend on aether_cli; CLI is the API boundary")

    # --- dynamic parity needs the built CLI; skip only that portion ---
    binary = _cli_binary(repo_root)
    skipped = False
    if binary is not None:
        try:
            output = subprocess.run(
                [str(binary), "discover", "commands", "--json"],
                check=True, capture_output=True, text=True, timeout=60,
            ).stdout
            discovered = json.loads(output)["commands"]
        except Exception as exc:
            problems.append(f"CLI discovery failed: {exc}")
        else:
            names = {c["name"] for c in discovered}
            if len(names) < 100:
                problems.append(f"CLI advertised only {len(names)} commands")
            missing = sorted(curated - names)
            if missing:
                problems.append(f"curated commands absent from CLI: {', '.join(missing)}")
            unmapped = sorted({name.split()[0] for name in names} - set(families))
            if unmapped:
                problems.append(f"CLI families without a workspace: {', '.join(unmapped)}")
            details.append(f"{len(names)} discovered commands, {len(curated)} curated, "
                           f"{len(families)} families mapped")
    else:
        skipped = True
        details.append("CLI binary absent: dynamic parity skipped (run cargo build)")

    if problems:
        return CheckResult(name="web_surface", ok=False,
                           summary=f"{len(problems)} web surface problem(s)",
                           details=problems + details)
    return CheckResult(
        name="web_surface",
        ok=True,
        skipped=skipped,
        summary="web assets air-gapped; no CLI dependency"
                + ("; dynamic parity skipped" if skipped else "; CLI/web parity verified"),
        details=details + ["10 workspaces, strict CSP, discovery-validated terminal"],
    )


if __name__ == "__main__":
    result = check()
    print(result.summary)
    for detail in result.details:
        print(" ", detail)
    raise SystemExit(0 if result.ok or result.skipped else 1)
