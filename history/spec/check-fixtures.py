#!/usr/bin/env python3
"""Check spec/r*.json against spec/UX-STATEMACHINE.md: every said-line must appear verbatim
(markdown markers, blockquote marks and table separators aside). Exit 1 on any miss.

The fixtures were extracted and adversarially audited by agents; this is the mechanical
backstop, so a paraphrase can never become a test.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def flatten(s: str) -> str:
    s = re.sub(r"[*`]", "", s).replace("\\|", "|")
    return re.sub(r"\s+", " ", s).strip()


def main() -> int:
    spec = (ROOT / "spec" / "UX-STATEMACHINE.md").read_text()
    haystacks = [
        flatten(spec),
        flatten(re.sub(r"^\s*>\s?", "", spec, flags=re.M).replace(" | ", " ").replace("<br>", " ")),
    ]
    total, missing = 0, []
    for f in sorted((ROOT / "spec").glob("r[0-9].json")):
        for state in json.loads(f.read_text())["states"]:
            for line in state["said_lines"]:
                total += 1
                if not any(flatten(line["text"]) in h for h in haystacks):
                    missing.append(f"{f.name} {state['id']}: {line['text'][:100]!r}")
    print(f"{total - len(missing)}/{total} said-lines verbatim in the spec")
    for m in missing:
        print("  missing:", m)
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
