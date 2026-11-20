#!/usr/bin/env python3
"""MCP サーバーを標準入出力で起動し、Claude Desktop と同じ手順で一通りのツールを呼ぶ動作確認。

使い方: python3 scripts/mcp_smoke.py target/debug/mondeep-mcp
"""
import json
import subprocess
import sys
import tempfile
import time
import os

binary = sys.argv[1]
db = os.path.join(tempfile.mkdtemp(), "smoke.db")
p = subprocess.Popen([binary, "--db", db], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
_id = 0


def rpc(method, params=None, notify=False):
    global _id
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    if not notify:
        _id += 1
        msg["id"] = _id
    p.stdin.write(json.dumps(msg) + "\n")
    p.stdin.flush()
    if notify:
        return None
    while True:
        line = p.stdout.readline()
        if not line:
            raise SystemExit("server closed")
        resp = json.loads(line)
        if resp.get("id") == _id:
            return resp


timings = {}


def call(name, args):
    t = time.perf_counter()
    r = rpc("tools/call", {"name": name, "arguments": args})
    timings[name] = max(timings.get(name, 0), (time.perf_counter() - t) * 1000)
    res = r["result"]
    body = json.loads(res["content"][0]["text"])
    return res.get("isError", False), body


init = rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "smoke", "version": "0"}})
assert init["result"]["serverInfo"]["name"] == "mondeep", init
rpc("notifications/initialized", notify=True)

tools = sorted(t["name"] for t in rpc("tools/list")["result"]["tools"])
print("tools:", tools)
assert tools == sorted(["start_session", "get_review_queue", "get_term_history", "record_question",
                        "record_attempt", "end_session", "list_weak_terms", "search"]), tools
prompts = sorted(p_["name"] for p_ in rpc("prompts/list")["result"]["prompts"])
print("prompts:", prompts)
guide = rpc("prompts/get", {"name": "diagram_guide"})["result"]["messages"][0]["content"]["text"]
assert "JSON Schema" in guide

err, s = call("start_session", {"format": "term_to_meaning", "theme": "XSS で面接形式の深掘り"})
assert not err, s
sid = s["session_id"]
err, q = call("record_question", {"session_id": sid, "prompt": "XSS とは何か説明してください", "term": "クロスサイトスクリプティング",
                                   "aliases": ["XSS"], "tags": ["Web", "面接"]})
assert not err, q
bad_diagram = {"version": 1, "kind": "concept", "nodes": [{"id": "a", "label": "攻撃者", "side": "error", "row": 0, "col": 0},
                                                         {"id": "b", "label": "被害者", "row": 0, "col": 1}],
               "edges": [{"from": "a", "to": "b", "label": ""}]}
err, r = call("record_attempt", {"question_id": q["question_id"], "answer": "スクリプトを埋め込む攻撃", "verdict": "partial",
                                  "reason": "反射型・格納型の区別がない", "explanation_markdown": "## XSS", "diagrams": [bad_diagram]})
assert err and r["violations"], r
print("diagram violation returned:", r["violations"][0])
bad_diagram["edges"][0]["label"] = "罠リンクを送る"
err, r = call("record_attempt", {"question_id": q["question_id"], "answer": "スクリプトを埋め込む攻撃", "verdict": "partial",
                                  "reason": "反射型・格納型の区別がない", "explanation_markdown": "## XSS\n仕組み…", "diagrams": [bad_diagram],
                                  "sources": [{"kind": "obsidian", "name": "Web セキュリティ/XSS"}]})
assert not err, r
print("next review:", r["next_review_date"])
err, c = call("record_question", {"session_id": sid, "prompt": "格納型と反射型の違いは？", "parent_question_id": q["question_id"]})
assert not err and c["depth"] == 1, c
call("record_attempt", {"question_id": c["question_id"], "answer": "わからない", "verdict": "incorrect", "reason": "未回答"})
err, amb = call("record_question", {"session_id": sid, "prompt": "?", "term": "クロスサイトスクリプティン"})
assert err and amb["candidates"], amb
err, e = call("end_session", {"session_id": sid, "summary": "深掘り 1 段目で詰まる"})
assert not err and e["question_count"] == 2, e
err, h = call("get_term_history", {"term": "xss"})
assert h["found"] and len(h["history"]["entries"]) == 2, h
err, w = call("list_weak_terms", {"limit": 5})
assert w["items"][0]["headword"] == "クロスサイトスクリプティング", w
print("weak reasons:", w["items"][0]["reasons"])
err, sr = call("search", {"keyword": "格納型"})
assert sr["count"] == 1, sr
err, rq = call("get_review_queue", {})
assert not err, rq
print("max latency ms:", {k: round(v, 1) for k, v in timings.items()})
assert all(v < 200 for v in timings.values()), timings
p.terminate()
print("OK")
