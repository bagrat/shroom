"""Dev only: a demo turn's stream-json (dev/cli/demo.sh) as a readable transcript.

    python3 dev/cli/transcript.py turn-1.jsonl [turn-2.jsonl …] > transcript.md
"""
import json
import os
import sys


def text_of(content):
    if isinstance(content, str):
        return content
    return "\n".join(x.get("text", "") for x in content if isinstance(x, dict))


for n, path in enumerate(sys.argv[1:], 1):
    prompt = os.path.join(os.path.dirname(path), f"prompt-{n}.txt")
    if os.path.exists(prompt):
        print(f"\n### You\n\n{open(prompt).read().strip()}\n")
    for line in open(path):
        e = json.loads(line)
        if e.get("type") == "user" and isinstance(e["message"]["content"], str):
            print(f"\n### You\n\n{e['message']['content']}\n")
        elif e.get("type") == "system" and e.get("subtype") == "init":
            pass
        elif e.get("type") == "assistant":
            for c in e["message"]["content"]:
                if c["type"] == "text":
                    print(f"\n**Agent:** {c['text']}\n")
                elif c["type"] == "tool_use":
                    print(f"```\n$ {c['input'].get('command', json.dumps(c['input']))}")
        elif e.get("type") == "user":
            for c in e["message"]["content"]:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    out = text_of(c["content"]).strip()
                    if len(out) > 700:
                        out = out[:700] + " …"
                    print(f"{out}\n```")
        elif e.get("type") == "result":
            print(f"\n<sub>turn {n}: {e.get('num_turns')} steps, {e.get('duration_ms', 0) / 1000:.0f} s</sub>\n")
