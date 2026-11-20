//! MCP の「プロンプト」で配る出題・採点・解説・図解のガイド。採点の質をそろえるために使う。
use crate::Config;

pub const SERVER_INSTRUCTIONS: &str = "Mondeep は Nico さんのセキュリティ用語学習の記録係です。\
出題・採点・解説づくりはあなた（Claude）が行い、その結果をこのサーバーのツールで保存します。\
学習を始める前に study_guide プロンプト（図を描くときは diagram_guide も）を読んでください。\
流れ: start_session → (get_review_queue / list_weak_terms / get_term_history で出題を選ぶ) → \
record_question → 回答を受けて record_attempt（判定・解説・図解・出典）→ 深掘りなら parent_question_id を付けて record_question → … → end_session。\
削除はできません（アプリ画面からのみ）。";

pub fn study_guide(cfg: &Config, format: Option<&str>) -> String {
    let materials = {
        let mut s = String::new();
        if let Some(v) = &cfg.obsidian_vault {
            s.push_str(&format!("- Obsidian の Vault「{v}」\n"));
        } else {
            s.push_str("- Obsidian のノート（Claude Desktop の既存の接続で読む）\n");
        }
        match &cfg.knowledge_path {
            Some(p) => s.push_str(&format!("- セキュリティのナレッジ DB: `{p}`\n")),
            None => s.push_str("- セキュリティのナレッジ DB（場所は未設定。Nico さんに確認する）\n"),
        }
        s.push_str("- 足りない場合は Web 検索の結果をもとにしてよい（出典に URL を残す）\n");
        s
    };
    let format_note = match format {
        Some("meaning_to_term") => "\n今回は **意味→用語** 形式です。\n",
        Some("term_to_meaning") => "\n今回は **用語→意味＋深掘り** 形式です。\n",
        _ => "",
    };
    format!(
        r#"# Mondeep 出題・採点ガイド
{format_note}
## 目的
Nico さん（28卒・セキュリティ職志望）が、セキュリティ用語を「説明でき、さらに深掘りされても答えられる」状態になること。
面接で聞かれる観点（仕組み・なぜ必要か・攻撃と対策・似た用語との違い）を意識する。

## 出題の素材
{materials}
保存時の sources に「どの素材由来か」を必ず残す（kind: obsidian / knowledge_db / web / other、name、web なら url）。

## 進め方
1. `start_session`（format: meaning_to_term / term_to_meaning / mixed、theme: 会話のテーマ）
2. 「今日の復習」なら `get_review_queue`、苦手克服なら `list_weak_terms` で出題する用語を選ぶ。
   出題前に `get_term_history` で過去の問いを確かめ、同じ問いの繰り返しを避ける。
3. 1 問ずつ `record_question` で保存してから出題する。
   - term には見出し語、aliases には略語・英語・カタカナの別表記、tags には分野（Web / 暗号 / ネットワーク / 面接 など）。
   - 「近い用語があります」と返ったら、候補が同じ用語なら term_id を指定、別の用語なら create_new_term=true で保存し直す。
4. 回答を受けたら判定し、`record_attempt` に回答文（チャットのまま）・判定・理由・解説・図解・出典をまとめて保存する。
   図解データが規約違反で返されたら、理由に沿って直して保存し直す。
5. 深掘り形式では、前の回答を踏まえた追加質問を parent_question_id を付けて `record_question` し、4 を繰り返す。
   詰まった段で無理に続けず、答えを解説して次の用語に移ってよい。
6. 終わったら `end_session` にまとめ（正答率、弱点、次に復習すべき観点）を保存する。

## 判定基準
- **意味→用語**: 用語が合っていれば正解。表記ゆれ（略語・英語・カタカナ・大文字小文字）は正解扱い。
  上位概念や近い用語なら部分正解（例: 「XSS」を問うて「インジェクション」）、別の用語なら不正解。
- **用語→意味**: 本質となる要素がすべて含まれれば正解、一部なら部分正解、本質を外していれば不正解。
  reason には「含まれていた要素」と「足りなかった要素」を具体的に書く。
- 深掘りの各段も同じ基準で判定する。

## 解説の書き方（Markdown）
- 1 行目に一言での定義。続けて「仕組み」「なぜ必要か／何が危ないか」「対策」「似た用語との違い」。
- 仕組みの図を 1 つ以上付ける（`diagram_guide` の図解データ。Mermaid・SVG・HTML は書かない）。
- 最後に「面接で聞かれやすい関連質問」を 2〜3 個、箇条書きで添える。
- 生の HTML は表示されない。リンクは Markdown のリンク記法で書く。
"#
    )
}

pub fn diagram_guide() -> String {
    let schema = serde_json::to_string_pretty(&mondeep_core::diagram::json_schema()).unwrap_or_default();
    format!(
        r#"# Mondeep 図解ガイド（図解データ v1）

Mermaid は使いません。座標や色は書かず「何を描くか」だけを JSON で渡し、見た目と配置はアプリが決めます。
`record_attempt` の diagrams に 0 個以上入れてください。保存前に検査され、違反は理由つきで返ります。

## 規約
- 全図共通で `"version": 1` と `"kind"`（concept / sequence / state / comparison）が必須。
- 色は side で決まる: `self`＝青（主役・自分側）、`external`＝オレンジ（外部）、`error`＝赤（エラー系・攻撃者）。
- 核心の値だけ `highlight: true`（ピンク）。1 図 2 箇所まで。
- **すべての矢印にラベル**（concept の edges[].label、sequence の messages[].label、state の transitions[].event）。番号は自動で付く。
- 1 図の要素は 7±2（部品・登場者・状態・行は最大 9）。多ければ図を分ける。
- 文字は短く: 部品名 全角 12 文字、役割 16 文字、矢印ラベル 14 文字程度まで（全角 2・半角 1 で数えて 24/32/28）。
- 主な流れは左→右（col を増やす）、時系列は上→下（row を増やす）。
- 配置: concept と state は部品を `row`/`col` のマス目に置く。同じマスに 2 つ置かない。
  マス目で収まらないときだけ `x`/`y`（中心座標 px、1 マス 220×130）で上書きできる。
- sequence と comparison は自動配置（位置は書かない）。sequence の枠（alt / opt / loop）は 1 つまで。
- 線の交差は警告として返る（保存はされる）。行・列を入れ替えて減らす。
- 凡例は自動で付く。

## 例
概念図（WAF の位置）:
```json
{{"version":1,"kind":"concept","title":"WAF の位置",
 "nodes":[{{"id":"user","label":"利用者","side":"external","row":0,"col":0}},
          {{"id":"atk","label":"攻撃者","side":"error","row":1,"col":0}},
          {{"id":"waf","label":"WAF","role":"HTTP の中身を検査","row":0,"col":1,"highlight":true}},
          {{"id":"app","label":"Web アプリ","row":0,"col":2}}],
 "edges":[{{"from":"user","to":"waf","label":"HTTP リクエスト"}},
          {{"from":"atk","to":"waf","label":"攻撃リクエスト","dashed":true}},
          {{"from":"waf","to":"app","label":"正常な通信だけ通す"}}],
 "groups":[{{"id":"own","label":"自社環境","members":["waf","app"]}}]}}
```
シーケンス図（TLS 1.3）:
```json
{{"version":1,"kind":"sequence","title":"TLS 1.3 フルハンドシェイク",
 "participants":[{{"id":"c","label":"クライアント"}},{{"id":"s","label":"サーバー","side":"external"}}],
 "messages":[{{"from":"c","to":"s","label":"ClientHello + key_share"}},
             {{"from":"s","to":"c","label":"ServerHello + key_share","style":"reply"}},
             {{"from":"s","to":"c","label":"証明書 + Finished","style":"reply","highlight":true}},
             {{"from":"c","to":"s","label":"Finished"}}]}}
```
状態遷移図: `{{"version":1,"kind":"state","states":[{{"id":"closed","label":"CLOSED","row":0,"col":0,"initial":true}},...],"transitions":[{{"from":"closed","to":"syn","event":"connect","action":"SYN 送信"}}]}}`

比較表: `{{"version":1,"kind":"comparison","columns":["共通鍵暗号","公開鍵暗号"],"rows":[{{"label":"鍵の数","cells":["1 つを共有","公開鍵と秘密鍵"]}}],"highlights":[{{"row":0,"col":1}}]}}`

## JSON Schema
```json
{schema}
```
"#
    )
}
