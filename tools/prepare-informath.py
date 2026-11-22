#!/usr/bin/env python3
"""Host-only preparation of pinned public Informath resources. Never model-selected."""

import argparse
import hashlib
import io
import json
import os
import ssl
import subprocess
import tarfile
import urllib.request
from pathlib import Path

RELEASE = (
    "https://github.com/GrammaticalFramework/informath/releases/download/informath-0.4/"
)
COMMIT = "cb11ebc4cf62d3563b547800b6e445c981f1178f"
ASSETS = [
    (
        "RunInformath-0.4-linux-x86.tgz",
        "207f9d0e845b4364fa018cd70ada9eeb3be1d78163c04646bd92b4f69c6b5615",
        "RunInformath",
    ),
    (
        "Informath-grammars-0.4.tgz",
        "d5228fda1e96618296d56462d892c50a9eb95ac270bdaaf4edb47a091ce025ca",
        "InformathEng.pgf",
    ),
]


def download(url):
    certificate = os.environ.get("SSL_CERT_FILE") or os.environ.get("NIX_SSL_CERT_FILE")
    if not certificate and Path("/etc/ssl/certs/ca-certificates.crt").is_file():
        certificate = "/etc/ssl/certs/ca-certificates.crt"
    context = ssl.create_default_context(cafile=certificate)
    return urllib.request.urlopen(url, timeout=60, context=context).read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out", type=Path, default=Path.home() / ".local/share/meden/informath/0.4"
    )
    args = parser.parse_args()
    root = args.out.resolve()
    (root / "share").mkdir(parents=True, exist_ok=True)
    for filename, digest, member in ASSETS:
        data = download(RELEASE + filename)
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError("Unexpected public release digest: " + filename)
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            entry = archive.getmember(member)
            if not entry.isfile():
                raise ValueError("Expected a regular release member")
            target = root / (member if member == "RunInformath" else "share/" + member)
            target.write_bytes(archive.extractfile(entry).read())
    executable = root / "RunInformath"
    executable.chmod(0o755)
    for name in ["baseconstants.dk", "baseconstants.dkgf"]:
        url = f"https://raw.githubusercontent.com/GrammaticalFramework/informath/{COMMIT}/share/{name}"
        (root / "share" / name).write_bytes(download(url))
    # NixOS has no FHS ELF loader. Retain only public installed dependency paths;
    # adapter execution still uses the ordinary offline sandbox and exact resources.
    if Path("/run/current-system/sw").is_dir():
        public = Path("/run/current-system/sw/bin")
        roots = [str(Path("/run/current-system/sw").resolve())]
        closure = subprocess.check_output(
            [str(public / "nix-store"), "--query", "--requisites", *roots],
            text=True,
            cwd="/",
        ).splitlines()
        libraries = []
        for pattern, filename in [
            ("*-ncurses-*", "libtinfo.so.6"),
            ("*-gmp-*", "libgmp.so.10"),
        ]:
            candidates = [Path(p) for p in closure if Path(p).match(pattern)]
            library = next(
                p / "lib" / filename
                for p in candidates
                if (p / "lib" / filename).is_file()
                and (p / "lib" / filename).read_bytes()[:5] == b"\x7fELF\x02"
            )
            libraries.append(str(library.parent))
        bash = (public / "bash").resolve(strict=True)
        patcher = str((public / "patchelf").resolve(strict=True))
        loader = subprocess.check_output(
            [patcher, "--print-interpreter", str(bash)], text=True
        ).strip()
        subprocess.run(
            [
                patcher,
                "--set-interpreter",
                loader,
                "--set-rpath",
                ":".join(libraries),
                str(executable),
            ],
            check=True,
        )
    subprocess.run(
        [str(executable), "-help"],
        env={**os.environ, "INFORMATH_ROOT": str(root)},
        check=True,
        capture_output=True,
        timeout=30,
    )
    paths = [
        "RunInformath",
        "share/InformathEng.pgf",
        "share/baseconstants.dk",
        "share/baseconstants.dkgf",
    ]
    manifest = {
        "version": "informath-0.4",
        "files": {
            p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths
        },
    }
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Prepared Informath 0.4 (BSD-3-Clause) and GF grammar: {root}")


if __name__ == "__main__":
    main()
