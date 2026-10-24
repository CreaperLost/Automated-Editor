#!/usr/bin/env bash
# run.sh <log-name> <prompt-file> [session-id]
# One headless EVAL-1 agent turn against the dev app's MCP server only (see evaluation.md).
# EVAL_DIR is the run's folder in scratch, holding mcp.json; the stream goes to
# $EVAL_DIR/logs/<log-name>.jsonl, then the turn's summary and what the agent read are printed.
E="${EVAL_DIR:?set EVAL_DIR to the run folder (with mcp.json)}"
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$E/agent-cwd" "$E/logs"
cd "$E/agent-cwd" || exit 1
resume=()
[ -n "$3" ] && resume=(--resume "$3")
env $(env | grep -oE "^(CLAUDE[A-Z_]*|ANTHROPIC[A-Z_]*|AI_AGENT|BAGGAGE)=" | sed "s/=\$//; s/^/-u /" | tr "
" " ") claude -p "$(cat "$2")" "${resume[@]}" \
  --model claude-sonnet-5-5 \
  --mcp-config "$E/mcp.json" --strict-mcp-config \
  --tools "" --allowedTools mcp__aeroedits \
  --setting-sources project --disable-slash-commands \
  --output-format stream-json --verbose \
  < /dev/null > "$E/logs/$1.jsonl" 2> "$E/logs/$1.err"
python -X utf8 - "$E/logs/$1.jsonl" <<'PY'
import json, sys
calls, result = [], None
for line in open(sys.argv[1], encoding="utf-8"):
    try: m = json.loads(line)
    except Exception: continue
    if m.get("type") == "assistant":
        for c in m["message"].get("content", []):
            if c.get("type") == "tool_use": calls.append(c["name"].replace("mcp__aeroedits__", ""))
    if m.get("type") == "result": result = m
print("calls:", len(calls), " ".join(calls))
if result:
    u = result.get("usage", {})
    print("session:", result.get("session_id"), "turns:", result.get("num_turns"),
          "ms:", result.get("duration_ms"), "cost $:", result.get("total_cost_usd"))
    print("tokens in/cache-read/cache-write/out:", u.get("input_tokens"), u.get("cache_read_input_tokens"),
          u.get("cache_creation_input_tokens"), u.get("output_tokens"))
    print("--- reply ---"); print(result.get("result", "")[:4000])
PY
# What the agent has read so far in this edit (all its turns' logs, in order).
python -X utf8 "$HERE/tokens.py" $(ls -tr "$E"/logs/*.jsonl)
