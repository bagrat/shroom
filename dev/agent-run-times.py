#!/usr/bin/env python3
"""Where a multi-agent run's time went: per agent, the time the model took and the time
its commands took, read from the run's own transcripts. The numbers behind
dev/AGENT-RUNS.md — run it after a run and add the row there.

    python3 dev/agent-run-times.py <run's transcript dir>…

A run's transcript dir is what the Workflow tool prints when it starts one ("Transcript
dir: …/subagents/workflows/wf_…"): it holds journal.jsonl and one transcript per agent.
Reads only; standard library only.

How it counts: in a transcript, the gap before an assistant message is the model's (it
was thinking or writing), and the gap before a tool's result is the command's. An agent
still running is counted up to its last line and marked.
"""
import glob
import json
import os
import sys
from datetime import datetime


def stamp(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00"))


def lines(path):
    with open(path) as f:
        for line in f:
            try:
                yield json.loads(line)
            except ValueError:
                continue


def agent_times(path):
    prev = prev_kind = first = None
    model = commands = 0.0
    turns = 0
    long_commands = []
    for entry in lines(path):
        at, kind = entry.get("timestamp"), entry.get("type")
        if not at or kind not in ("assistant", "user"):
            continue
        at = stamp(at)
        first = first or at
        if prev is not None:
            gap = (at - prev).total_seconds()
            if kind == "assistant":
                model += gap
                turns += prev_kind != "assistant"
            else:
                commands += gap
                if gap >= 240:
                    long_commands.append(gap / 60)
        prev, prev_kind = at, kind
    return first, prev, model, commands, turns, sorted(long_commands, reverse=True)


def run(directory):
    labels, finished = {}, set()
    for entry in lines(os.path.join(directory, "journal.jsonl")):
        if entry.get("type") == "started":
            labels[entry["agentId"]] = entry.get("label") or entry["agentId"]
        elif entry.get("type") == "result":
            finished.add(entry["agentId"])
    rows = []
    for agent, label in labels.items():
        found = glob.glob(os.path.join(directory, f"*{agent}*.jsonl"))
        if not found:
            continue
        first, last, model, commands, turns, long_commands = agent_times(found[0])
        if first is None:
            continue
        rows.append((first, label, last, model, commands, turns, long_commands, agent in finished))
    rows.sort(key=lambda row: row[0])
    if not rows:
        print(f"{directory}: no transcripts")
        return 0.0, 0.0
    print(f"\n{os.path.basename(os.path.normpath(directory))}  "
          f"{rows[0][0].astimezone():%Y-%m-%d %H:%M} to {max(r[2] for r in rows).astimezone():%H:%M}")
    print(f"  {'agent':28} {'total':>6} {'model':>6} {'cmds':>6} {'turns':>5} {'s/turn':>6}  commands of 4 min or more")
    total_model = total_commands = 0.0
    for first, label, last, model, commands, turns, long_commands, done in rows:
        total_model += model
        total_commands += commands
        mark = "" if done else "  (not finished)"
        print(f"  {label:28} {(model + commands) / 60:5.0f}m {model / 60:5.0f}m {commands / 60:5.0f}m "
              f"{turns:5d} {model / max(turns, 1):6.1f}  {' '.join(f'{m:.0f}' for m in long_commands)}{mark}")
    both = total_model + total_commands
    print(f"  {'all agents':28} {both / 60:5.0f}m {total_model / 60:5.0f}m {total_commands / 60:5.0f}m"
          f"   the model's share: {100 * total_model / both:.0f}%")
    return total_model, total_commands


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    model = commands = 0.0
    for directory in sys.argv[1:]:
        m, c = run(directory)
        model, commands = model + m, commands + c
    if len(sys.argv) > 2 and model + commands:
        print(f"\nall runs: model {model / 3600:.1f} h, commands {commands / 3600:.1f} h, "
              f"the model's share {100 * model / (model + commands):.0f}%")
