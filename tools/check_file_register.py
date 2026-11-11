#!/usr/bin/env python3
"""Invarianten-Test fuer FILE_REGISTER.md.

Prueft das Register gegen den echten Git-Baum, damit eine Spezifikationsdatei
nicht unbemerkt herausfaellt (am 10.11.2026 sind Blueprint und Messprotokoll
genau so verschwunden).

Geprueft wird:
  1. Mengengleichheit Dateiliste <-> `git ls-files`
  2. deterministische Sortierung der Liste
  3. Verzeichniszahl in der Tabelle <-> Baum (alle Ebenen)
  4. Kopf vermerkt den Erzeugungs- oder Standzeitpunkt
  5. jede Datei unter docs/specs/ steht im Register (explizite Regel)
  6. die Zahlen der Zusammenfassungstabelle stimmen je Endung

Aufruf:  python3 tools/check_file_register.py [--root .]
Exit 0 = alle Invarianten halten. Exit 1 = mindestens eine verletzt.
"""
from __future__ import annotations

import argparse
import collections
import os
import subprocess
import sys


def git_files(root: str) -> list[str]:
    out = subprocess.run(["git", "ls-files"], cwd=root, capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit("git ls-files fehlgeschlagen: " + out.stderr.strip())
    return out.stdout.splitlines()


def all_dirs(paths: list[str]) -> set[str]:
    dirs: set[str] = set()
    for f in paths:
        parts = f.split("/")[:-1]
        for i in range(1, len(parts) + 1):
            dirs.add("/".join(parts[:i]))
    return dirs


def ext_of(path: str) -> str:
    base = path.rsplit("/", 1)[-1]
    if base.startswith(".") and base.count(".") == 1:
        return base
    if "." not in base:
        return "(keine)"
    return "." + base.rsplit(".", 1)[1]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=".")
    args = ap.parse_args()
    root = args.root
    reg_path = os.path.join(root, "FILE_REGISTER.md")
    if not os.path.exists(reg_path):
        sys.exit("FILE_REGISTER.md nicht gefunden in " + root)

    text = open(reg_path, encoding="utf-8").read()
    lines = text.split("\n")
    problems: list[str] = []

    # Format erkennen: typisiertes Register `| \`./pfad\` | n | typ |`
    typed = [l for l in lines if l.startswith("| `./")]
    bullets = [l[3:-1] for l in lines if l.startswith("- `")]
    if typed:
        listed = [l.split("`")[1][2:] for l in typed]
        mode = "typisiert"
    else:
        listed = bullets
        mode = "Liste"

    files = git_files(root)
    set_listed, set_files = set(listed), set(files)

    missing = sorted(set_files - set_listed)
    extra = sorted(set_listed - set_files)
    if missing:
        problems.append(f"{len(missing)} Datei(en) fehlen im Register: {missing[:5]}")
    if extra:
        problems.append(f"{len(extra)} Registereintrag/-eintraege ohne Datei: {extra[:5]}")

    if mode == "Liste" and listed != sorted(listed):
        problems.append("Dateiliste ist nicht sortiert")

    if len(listed) != len(set(listed)):
        problems.append("Dateiliste enthaelt Duplikate")

    # Verzeichniszahl gegen den Baum
    want_dirs = len(all_dirs(files))
    for l in lines:
        if "Verzeichnisse**" in l or "Verzeichnisse |" in l:
            num = "".join(c for c in l.split("|")[2] if c.isdigit()) if l.startswith("|") else ""
            if num and int(num) != want_dirs:
                problems.append(f"Verzeichniszahl {num} != {want_dirs} im Baum")

    # Standvermerk im Kopf
    if not any(("**Stand:**" in l or "**Neu erzeugt:**" in l or "**Erzeugt:**" in l) for l in lines[:15]):
        problems.append("Kopf enthaelt keinen Erzeugungs- oder Standvermerk")

    # Spezifikationen muessen im Register stehen
    specs = [f for f in files if f.startswith("docs/specs/")]
    for s in specs:
        if s not in set_listed:
            problems.append(f"Spezifikation nicht im Register: {s}")

    # Zahlentabelle je Endung
    ist = collections.Counter(ext_of(f) for f in files)
    for l in lines:
        if not l.startswith("| **"):
            continue
        cells = [c.strip() for c in l.strip("|").split("|")]
        if len(cells) < 2 or not cells[1].isdigit():
            continue
        label = cells[0].strip("* ").strip()
        if label.lower().startswith("total"):
            if int(cells[1]) != len(files):
                problems.append(f"Total {cells[1]} != {len(files)} Dateien im Baum")
        elif label in ist or label.startswith("."):
            if int(cells[1]) != ist.get(label, 0):
                problems.append(f"{label}: Tabelle {cells[1]} != {ist.get(label, 0)} im Baum")

    print(f"Register: {os.path.relpath(reg_path)}  Format: {mode}")
    print(f"Eintraege: {len(listed)}  Dateien im Baum: {len(files)}  Verzeichnisse: {want_dirs}")
    if problems:
        print(f"\n{len(problems)} Verletzung(en):")
        for p in problems:
            print("  - " + p)
        return 1
    print("Alle Invarianten halten.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
