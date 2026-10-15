#!/usr/bin/env python3
"""Differential-test the two dependency resolvers over the EDSP protocol.

Runs the same real apt request through both resolvers and compares the
solution stanzas:

  - C++  side: apt's internal solver (solver3, /usr/libexec/apt/solvers/apt)
  - Rust side: oma-edsp (the oma-apt-pkg resolvo-based external solver)

Both consume identical EDSP input, so the diff isolates resolver behavior
from data. The Install/Remove sets are the "resolution" and must match;
Autoremove stanzas are reported separately (oma-edsp reports newly
auto-installed packages, apt's solver3 currently does not).

Usage:
  python3 scripts/compare-resolvers.py install lolcat
  python3 scripts/compare-resolvers.py remove tree
  python3 scripts/compare-resolvers.py upgrade
  python3 scripts/compare-resolvers.py dist-upgrade

Env overrides:
  OMA_EDSP        path to the oma-edsp binary (default: repo target/debug|release)
  APT_SOLVER      path to apt's internal solver (default: /usr/libexec/apt/solvers/apt
                  or /usr/lib/apt/solvers/apt)
  APT_SOLVERS_DIR extra dir for apt to find the capture solver (optional)

Exit code 0 when Install/Remove match, 1 when they differ, 2 on setup error.
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def find_oma_edsp() -> Path:
    env = os.environ.get("OMA_EDSP")
    if env:
        return Path(env)
    for profile in ("release", "debug"):
        p = REPO / "target" / profile / "oma-edsp"
        if p.is_file():
            return p
    # Build the debug binary on first use.
    subprocess.run(
        ["cargo", "build", "-p", "oma-apt-pkg", "--bin", "oma-edsp"],
        cwd=REPO,
        check=True,
    )
    return REPO / "target" / "debug" / "oma-edsp"


def find_apt_solver() -> Path:
    env = os.environ.get("APT_SOLVER")
    if env:
        return Path(env)
    for p in ("/usr/libexec/apt/solvers/apt", "/usr/lib/apt/solvers/apt"):
        if Path(p).is_file():
            return Path(p)
    raise SystemExit("apt internal solver not found; set APT_SOLVER")


def capture_request(apt_args: list[str]) -> Path:
    """Run apt-get in simulate mode with a capture solver that dumps the
    EDSP request to a file, and return that file."""
    workdir = Path(tempfile.mkdtemp(prefix="oma-resolver-cmp-"))
    solvers_dir = workdir / "solvers"
    solvers_dir.mkdir()
    request = workdir / "request.txt"
    capture = solvers_dir / "capture"
    capture.write_text(f"#!/bin/sh\ncat > {request}\n")
    capture.chmod(0o755)

    proc = subprocess.run(
        [
            "apt-get",
            "-s",
            f"-o=Dir::Bin::solvers={solvers_dir}",
            "-o=APT::Solver=capture",
            *apt_args,
        ],
        capture_output=True,
        text=True,
    )
    if not request.is_file():
        # apt usually rejects the request (unknown package, ...) before ever
        # invoking the solver; show its message instead of a bare failure.
        print(proc.stdout, end="", file=sys.stderr)
        print(proc.stderr, end="", file=sys.stderr)
        raise SystemExit(f"apt did not invoke the capture solver for '{' '.join(apt_args)}'")
    return request


def idmap(request: Path) -> dict[int, str]:
    """Map APT-ID -> package name from the EDSP universe."""
    m: dict[int, str] = {}
    name: str | None = None
    for line in request.read_text(encoding="utf-8").splitlines():
        if line.startswith("Package: "):
            name = line[9:]
        elif line.startswith("APT-ID: ") and name is not None:
            m[int(line[8:])] = name
    return m


def solution(output: Path, idmap_: dict[int, str]) -> dict[str, list[str]]:
    res = {"Install": [], "Remove": [], "Autoremove": []}
    for line in output.read_text(encoding="utf-8").splitlines():
        for kind in res:
            prefix = kind + ": "
            if line.startswith(prefix):
                res[kind].append(idmap_.get(int(line[len(prefix) :]), "?" + line[len(prefix) :]))
    return res


def run_solver(binary: Path, request: Path, out: Path) -> None:
    with request.open("rb") as fin, out.open("wb") as fout:
        subprocess.run([str(binary)], stdin=fin, stdout=fout, stderr=subprocess.DEVNULL)


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    apt_args = sys.argv[1:]

    request = capture_request(apt_args)
    ids = idmap(request)

    cpp = find_apt_solver()
    rust = find_oma_edsp()
    out_apt = request.parent / "out_apt.txt"
    out_oma = request.parent / "out_oma.txt"
    run_solver(cpp, request, out_apt)
    run_solver(rust, request, out_oma)

    a = solution(out_apt, ids)
    r = solution(out_oma, ids)

    print(f"== apt-get {' '.join(apt_args)} ==")
    print(f"   C++ solver : {cpp}")
    print(f"   Rust solver: {rust}")
    ok = True
    for kind in ("Install", "Remove"):
        sa, sr = sorted(a[kind]), sorted(r[kind])
        same = sa == sr
        ok = ok and same
        print(f"   {kind:10} {'SAME' if same else 'DIFF'}")
        for x in sa:
            print(f"     apt {kind}: {x}")
        for x in sr:
            print(f"     oma {kind}: {x}")
    # Autoremove is informational: oma-edsp reports newly auto-installed
    # packages, apt's solver3 currently does not.
    print(
        f"   Autoremove oma: {sorted(r['Autoremove'])}"
        + ("   (apt: none)" if not a["Autoremove"] else f"   apt: {sorted(a['Autoremove'])}")
    )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
