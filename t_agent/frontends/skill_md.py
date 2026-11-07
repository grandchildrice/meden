"""Frontend for Agent Skills packages (``SKILL.md`` + scripts).

Reads the frontmatter used by Claude Code, pi, Hermes and OpenClaw (the
agentskills.io specification), including OpenClaw's
``metadata.openclaw.requires`` block, and scans bundled scripts for effects.

The static scan is a frontend heuristic that *over-approximates* what the
skill needs. Over-approximation is the safe direction here: a skill that the
scan says needs ``shell.exec`` and ``net.egress`` must either be granted
those or be rejected. Under-approximation (missing an effect) is the failure
mode this prototype will replace with real typing of script languages; the
regexes below are a stand-in, not a boundary.
"""

from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

import yaml

from ..capability import Capability, Requirement

SCRIPT_SUFFIXES = {".sh", ".bash", ".zsh", ".py", ".js", ".mjs", ".ts", ".rb", ".pl", ""}

# (capability, scope-or-None, pattern) applied line by line to scripts and to
# the SKILL.md body. Patterns are deliberately broad.
_SCRIPT_PATTERNS: list[tuple[Capability, str | None, re.Pattern[str]]] = [
    (Capability.NET_EGRESS, None, re.compile(r"\b(curl|wget|Invoke-WebRequest|iwr)\b")),
    (Capability.NET_EGRESS, None, re.compile(r"\b(requests|httpx|urllib|aiohttp|fetch)\s*[\.(]")),
    (Capability.SHELL_EXEC, None, re.compile(r"\|\s*(sudo\s+)?(ba|z|)sh\b")),  # curl ... | bash
    (Capability.SHELL_EXEC, None, re.compile(r"\b(subprocess|os\.system|child_process|exec\s*\()")),
    (
        Capability.PACKAGE_INSTALL,
        None,
        re.compile(r"\b(pip|pip3|npm|npx|pnpm|yarn|brew|apt(-get)?|uv)\s+(install|add|i)\b"),
    ),
    (
        Capability.SECRET_READ,
        None,
        re.compile(
            r"(\.ssh/|id_rsa|id_ed25519|\.aws/credentials|\.netrc|keychain|Keychain|wallet)"
        ),
    ),
    (
        Capability.ENV_READ,
        None,
        re.compile(r"(\$\{?[A-Z][A-Z0-9_]*(KEY|TOKEN|SECRET|PASSWORD)\b|os\.environ|process\.env)"),
    ),
    (
        Capability.FS_WRITE,
        None,
        re.compile(r"(>\s*/|\btee\b|\bchmod\b|\bcrontab\b|\blaunchctl\b|\bsystemctl\b)"),
    ),
    (Capability.PROCESS_SPAWN, None, re.compile(r"\b(nohup|setsid|daemonize|&\s*$)")),
]

_BINARY_CAPS: dict[str, Capability] = {
    "curl": Capability.NET_EGRESS,
    "wget": Capability.NET_EGRESS,
    "ssh": Capability.NET_EGRESS,
    "docker": Capability.PROCESS_SPAWN,
    "uv": Capability.PACKAGE_INSTALL,
    "pip": Capability.PACKAGE_INSTALL,
    "npm": Capability.PACKAGE_INSTALL,
    "brew": Capability.PACKAGE_INSTALL,
}


@dataclass
class SkillPackage:
    path: Path
    name: str
    description: str
    frontmatter: dict = field(default_factory=dict)
    body: str = ""
    files: list[Path] = field(default_factory=list)


def load_skill(path: str | Path) -> SkillPackage:
    root = Path(path)
    skill_md = root / "SKILL.md"
    if not skill_md.is_file():
        raise FileNotFoundError(f"no SKILL.md in {root}")
    text = skill_md.read_text(encoding="utf-8")
    fm, body = _split_frontmatter(text)
    files = sorted(p for p in root.rglob("*") if p.is_file() and p != skill_md)
    return SkillPackage(
        path=root,
        name=str(fm.get("name", root.name)),
        description=str(fm.get("description", "")),
        frontmatter=fm,
        body=body,
        files=files,
    )


def extract_requirements(pkg: SkillPackage) -> set[Requirement]:
    reqs: set[Requirement] = set()
    reqs |= _from_openclaw_metadata(pkg)
    reqs |= _scan_text(pkg.body, "SKILL.md")
    for f in pkg.files:
        if f.suffix.lower() in SCRIPT_SUFFIXES or _looks_like_script(f):
            try:
                content = f.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            reqs |= _scan_text(content, str(f.relative_to(pkg.path)))
    return reqs


# -- helpers ----------------------------------------------------------------


def _split_frontmatter(text: str) -> tuple[dict, str]:
    m = re.match(r"^---\s*\n(.*?)\n---\s*\n?(.*)$", text, re.DOTALL)
    if not m:
        return {}, text
    raw, body = m.group(1), m.group(2)
    try:
        fm = yaml.safe_load(raw) or {}
    except yaml.YAMLError:
        fm = {}
    if not isinstance(fm, dict):
        fm = {}
    return fm, body


def _openclaw_block(fm: dict) -> dict:
    meta = fm.get("metadata")
    if isinstance(meta, str):
        # OpenClaw writes this block as JSON5; plain JSON and YAML are the
        # common cases. Anything else is treated as empty metadata.
        for loader in (json.loads, yaml.safe_load):
            try:
                parsed = loader(meta)
                if isinstance(parsed, dict):
                    meta = parsed
                    break
            except Exception:  # noqa: BLE001 - loader-specific errors
                continue
        else:
            return {}
    if not isinstance(meta, dict):
        return {}
    oc = meta.get("openclaw", meta)
    return oc if isinstance(oc, dict) else {}


def _from_openclaw_metadata(pkg: SkillPackage) -> set[Requirement]:
    reqs: set[Requirement] = set()
    oc = _openclaw_block(pkg.frontmatter)
    requires = oc.get("requires", {}) if isinstance(oc.get("requires"), dict) else {}
    for b in list(requires.get("bins", [])) + list(requires.get("anyBins", [])):
        cap = _BINARY_CAPS.get(str(b), Capability.SHELL_EXEC)
        reqs.add(Requirement(cap, str(b), evidence="metadata.openclaw.requires.bins"))
    for env in requires.get("env", []):
        cap = (
            Capability.SECRET_READ
            if re.search(r"KEY|TOKEN|SECRET|PASS", str(env))
            else Capability.ENV_READ
        )
        reqs.add(Requirement(cap, str(env), evidence="metadata.openclaw.requires.env"))
    for spec in oc.get("install", []) or []:
        if isinstance(spec, dict):
            kind = str(spec.get("kind", spec.get("type", "download")))
            reqs.add(
                Requirement(Capability.PACKAGE_INSTALL, kind, evidence="metadata.openclaw.install")
            )
            reqs.add(Requirement(Capability.NET_EGRESS, None, evidence="metadata.openclaw.install"))
    return reqs


def _scan_text(content: str, where: str) -> set[Requirement]:
    reqs: set[Requirement] = set()
    for lineno, line in enumerate(content.splitlines(), 1):
        for cap, scope, pat in _SCRIPT_PATTERNS:
            if pat.search(line):
                reqs.add(Requirement(cap, scope, evidence=f"{where}:{lineno}"))
    return reqs


def _looks_like_script(path: Path) -> bool:
    try:
        with path.open("rb") as fh:
            return fh.read(2) == b"#!"
    except OSError:
        return False


def main(argv: list[str] | None = None) -> int:
    args = argv if argv is not None else sys.argv[1:]
    if len(args) != 1:
        print("usage: python -m t_agent.frontends.skill_md <skill-dir>", file=sys.stderr)
        return 2
    pkg = load_skill(args[0])
    reqs = extract_requirements(pkg)
    print(f"{pkg.name}: {len(reqs)} requirement(s)")
    for r in sorted(reqs, key=str):
        print(f"  {r}  <- {r.evidence}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
