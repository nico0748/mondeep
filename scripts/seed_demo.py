#!/usr/bin/env python3
"""デモ用の学習記録を、Claude Desktop と同じく MCP 経由で DB に書き込む。

使い方: python3 scripts/seed_demo.py target/debug/mondeep-mcp /tmp/demo.db
アプリを MONDEEP_DB=/tmp/demo.db で起動すると、記録がある状態の画面を確かめられる。
"""
import json
import subprocess
import sys

binary, db = sys.argv[1], sys.argv[2]
p = subprocess.Popen([binary, "--db", db], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
_id = 0


def rpc(method, params=None, notify=False):
    global _id
    msg = {"jsonrpc": "2.0", "method": method, **({"params": params} if params is not None else {})}
    if not notify:
        _id += 1
        msg["id"] = _id
    p.stdin.write(json.dumps(msg, ensure_ascii=False) + "\n")
    p.stdin.flush()
    if notify:
        return None
    while True:
        r = json.loads(p.stdout.readline())
        if r.get("id") == _id:
            return r


def call(name, **args):
    res = rpc("tools/call", {"name": name, "arguments": args})["result"]
    body = json.loads(res["content"][0]["text"])
    if res.get("isError"):
        raise SystemExit(f"{name}: {body}")
    return body


rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "seed", "version": "0"}})
rpc("notifications/initialized", notify=True)

# 1. 用語→意味＋深掘り（XSS）
s = call("start_session", format="term_to_meaning", theme="XSS で面接形式の深掘り")["session_id"]
root = call("record_question", session_id=s, prompt="XSS（クロスサイトスクリプティング）とは何か、説明してください。",
            term="クロスサイトスクリプティング", aliases=["XSS", "Cross-Site Scripting"], tags=["Web", "面接"],
            model_answer="利用者の入力をエスケープせずにページへ出力することで、攻撃者のスクリプトが被害者のブラウザで実行される脆弱性。")
call("record_attempt", question_id=root["question_id"], answer="Web ページにスクリプトを埋め込んで、見た人のブラウザで動かす攻撃です。",
     verdict="correct", reason="「入力がそのまま出力される」「被害者のブラウザで実行される」という本質が両方含まれている。",
     explanation_markdown="""**一言で**: 入力値をエスケープせずに HTML へ出力したせいで、攻撃者の JavaScript が被害者のブラウザで動く脆弱性。

## 仕組み
1. 攻撃者が `<script>` を含む文字列を、罠リンクや投稿として仕込む
2. 脆弱なアプリがそれを **そのまま** ページに出力する
3. 被害者のブラウザが、そのサイトのオリジンでスクリプトを実行する（Cookie 窃取・なりすまし操作）

## 対策
- 出力箇所に応じたエスケープ（HTML・属性・JavaScript・URL）
- Content-Security-Policy で inline script を禁止
- Cookie に `HttpOnly` を付ける

## 面接で聞かれやすい関連質問
- 反射型・格納型・DOM-based の違いは？
- CSRF との違いは？
- CSP はなぜ XSS の緩和策になるのか？

参考: [OWASP Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html)""",
     diagrams=[{
         "version": 1, "kind": "concept", "title": "反射型 XSS の流れ",
         "nodes": [
             {"id": "atk", "label": "攻撃者", "side": "error", "row": 0, "col": 0},
             {"id": "victim", "label": "被害者のブラウザ", "role": "罠リンクを踏む", "row": 1, "col": 0},
             {"id": "app", "label": "脆弱な Web アプリ", "role": "入力をそのまま出力", "side": "external", "row": 1, "col": 2, "highlight": True},
         ],
         "edges": [
             {"from": "atk", "to": "victim", "label": "罠リンクを送る"},
             {"from": "victim", "to": "app", "label": "スクリプト入りの要求"},
             {"from": "app", "to": "victim", "label": "スクリプトを含む HTML", "dashed": True},
         ],
     }],
     sources=[{"kind": "obsidian", "name": "Web セキュリティ/XSS"},
              {"kind": "web", "name": "OWASP XSS Prevention Cheat Sheet", "url": "https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html"}])
c1 = call("record_question", session_id=s, prompt="反射型 XSS と格納型 XSS の違いは？", parent_question_id=root["question_id"])
call("record_attempt", question_id=c1["question_id"], answer="反射型はリンクを踏ませる、格納型はサーバーに保存されて見た人全員に効く。",
     verdict="correct", reason="スクリプトの置き場所（要求の中か、サーバーの保存データか）の違いを押さえている。",
     explanation_markdown="格納型は掲示板の投稿などに保存され、**閲覧した全員** が被害を受けるため影響が大きい。",
     diagrams=[{"version": 1, "kind": "comparison", "title": "XSS の種類",
                "columns": ["反射型", "格納型", "DOM-based"],
                "rows": [{"label": "スクリプトの置き場所", "cells": ["要求（URL など）", "サーバーの保存データ", "クライアント側の処理"]},
                         {"label": "被害者", "cells": ["罠を踏んだ人", "閲覧した全員", "罠を踏んだ人"]},
                         {"label": "サーバーの関与", "cells": ["反射して返す", "保存して配る", "関与しないこともある"]}],
                "highlights": [{"row": 1, "col": 1}]}])
c2 = call("record_question", session_id=s, prompt="CSP の nonce 方式は、なぜ XSS を防げるのですか？", parent_question_id=c1["question_id"])
call("record_attempt", question_id=c2["question_id"], answer="許可したスクリプトだけ動くから……？", verdict="partial",
     reason="「許可したスクリプトだけ」は正しいが、nonce が毎回ランダムで攻撃者が予測できない点が抜けている。",
     explanation_markdown="nonce はレスポンスごとにランダムに生成され、攻撃者が注入したスクリプトには正しい nonce を付けられないため実行されない。")
call("end_session", session_id=s, summary="定義と種類は説明できた。CSP の nonce が予測できないことが核心、という点で詰まった。")

# 2. 意味→用語（TLS 周り）
s2 = call("start_session", format="meaning_to_term", theme="TLS 周りを意味→用語で")["session_id"]
items = [
    ("TLS 1.3", ["TLS1.3"], "TLS の最新版。ハンドシェイクを 1-RTT に短縮し、前方秘匿性のない鍵交換を廃止した。", "TLS1.3", "correct"),
    ("前方秘匿性", ["Forward Secrecy", "PFS"], "サーバーの秘密鍵が後から漏れても、過去の通信が復号されない性質。", "PFS", "correct"),
    ("証明書の透明性", ["Certificate Transparency", "CT"], "発行されたサーバー証明書を公開ログに記録し、不正発行を検知できるようにする仕組み。", "OCSP", "incorrect"),
    ("HSTS", ["HTTP Strict Transport Security"], "ブラウザに、そのサイトへは常に HTTPS で接続するよう指示するレスポンスヘッダー。", "HTTPS の強制…名前が出ない", "partial"),
]
for head, aliases, meaning, ans, verdict in items:
    q = call("record_question", session_id=s2, prompt=meaning, term=head, aliases=aliases, tags=["暗号", "ネットワーク"], create_new_term=True)
    diagrams = []
    if head == "TLS 1.3":
        diagrams = [{"version": 1, "kind": "sequence", "title": "TLS 1.3 フルハンドシェイク",
                     "participants": [{"id": "c", "label": "クライアント"}, {"id": "s", "label": "サーバー", "side": "external"}],
                     "messages": [{"from": "c", "to": "s", "label": "ClientHello + key_share"},
                                  {"from": "s", "to": "c", "label": "ServerHello + key_share", "style": "reply"},
                                  {"from": "s", "to": "c", "label": "証明書・Finished（暗号化済み）", "style": "reply", "highlight": True},
                                  {"from": "c", "to": "s", "label": "Finished"},
                                  {"from": "c", "to": "s", "label": "アプリケーションデータ", "style": "async"}],
                     "fragment": {"kind": "opt", "label": "クライアント認証", "start": 2, "end": 3}}]
    call("record_attempt", question_id=q["question_id"], answer=ans, verdict=verdict,
         reason="用語が一致" if verdict == "correct" else "別の仕組み（失効確認）と混同" if verdict == "incorrect" else "仕組みは分かっているが用語が出ない",
         explanation_markdown=f"**{head}**: {meaning}", diagrams=diagrams)
call("end_session", session_id=s2, summary="CT と OCSP を混同。証明書まわりの用語を復習する。")

# 3. TCP の状態遷移（進行中）
s3 = call("start_session", format="term_to_meaning", theme="ネットワークの基礎")["session_id"]
q = call("record_question", session_id=s3, prompt="TCP の 3 ウェイハンドシェイクを説明してください。", term="3 ウェイハンドシェイク",
         aliases=["スリーウェイハンドシェイク", "3-way handshake"], tags=["ネットワーク"])
call("record_attempt", question_id=q["question_id"], answer="SYN、SYN/ACK、ACK の 3 回のやり取りで接続を確立する。", verdict="correct",
     reason="3 つのセグメントと順番が正しい。",
     explanation_markdown="シーケンス番号の初期値を互いに伝え合うことが目的。",
     diagrams=[{"version": 1, "kind": "state", "title": "クライアント側の TCP 状態",
                "states": [{"id": "closed", "label": "CLOSED", "row": 0, "col": 0, "initial": True},
                           {"id": "syn", "label": "SYN_SENT", "row": 0, "col": 1},
                           {"id": "est", "label": "ESTABLISHED", "row": 0, "col": 2, "highlight": True},
                           {"id": "fin", "label": "FIN_WAIT", "row": 1, "col": 2},
                           {"id": "end", "label": "TIME_WAIT", "row": 1, "col": 1, "final": True}],
                "transitions": [{"from": "closed", "to": "syn", "event": "connect", "action": "SYN 送信"},
                                {"from": "syn", "to": "est", "event": "SYN+ACK 受信", "action": "ACK 送信"},
                                {"from": "est", "to": "fin", "event": "close", "action": "FIN 送信"},
                                {"from": "fin", "to": "end", "event": "FIN 受信", "action": "ACK 送信"}]}])
p.terminate()
print("seeded", db)
