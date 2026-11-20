//! 図解フレームワークのデータ形式（スキーマ v1）と検査規則。
//!
//! Claude は座標や色を書かず「何を描くか」だけを図解データ（JSON）で渡す。
//! MCP サーバーは保存前にここでスキーマと規約を検査し、違反があれば理由を返して Claude に直させる。
//! 描画はアプリ側（`app/src/zukai`）が同じ規約で行う。AI が書いた SVG・HTML は受け付けない。
//!
//! 図の種類を足すときは、`Diagram` に列挙子を足し、`validate_*` を書き、アプリ側に描画を足す。
use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::normalize::display_width;

/// 現在の図解データの版。古い版のデータは `upgrade` で読み替えて描けるようにする。
pub const SCHEMA_VERSION: u32 = 1;
/// 1 図の要素数の上限（7±2 の上限）。
pub const MAX_ELEMENTS: usize = 9;
pub const MAX_MESSAGES: usize = 16;
pub const MAX_GRID: u32 = 8;
/// マス目 1 つの大きさ（px）。アプリ側の描画と同じ値。
pub const CELL_W: f64 = 220.0;
pub const CELL_H: f64 = 130.0;

pub const MAX_LABEL_WIDTH: usize = 24;
pub const MAX_ROLE_WIDTH: usize = 32;
pub const MAX_EDGE_LABEL_WIDTH: usize = 28;
pub const MAX_TITLE_WIDTH: usize = 48;
pub const MAX_CELL_WIDTH: usize = 48;
pub const MAX_HIGHLIGHTS: usize = 2;

/// 部品の立ち位置。色は アプリ側で決める（self=青、external=オレンジ、error=赤）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// 主役・自分側（青）
    #[default]
    #[serde(rename = "self")]
    Own,
    /// 外部（オレンジ）
    External,
    /// エラー系・攻撃者（赤）
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Diagram {
    /// 概念図（構成図スタイル）: 仕組みや登場人物の関係
    Concept(ConceptDiagram),
    /// シーケンス図: やり取りの順番
    Sequence(SequenceDiagram),
    /// 状態遷移図: 状態の変化
    State(StateDiagram),
    /// 比較表: 似た用語の違い
    Comparison(ComparisonDiagram),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConceptDiagram {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub nodes: Vec<ConceptNode>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    /// 部品をまとめる枠（例: 「社内ネットワーク」）
    #[serde(default)]
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConceptNode {
    pub id: String,
    /// 部品名
    pub label: String,
    /// 役割 1 行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub side: Side,
    /// マス目の行（0 始まり、上→下）
    pub row: u32,
    /// マス目の列（0 始まり、主な流れは左→右）
    pub col: u32,
    /// マス目で収まらないときだけ、中心座標（px）で上書きする
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    /// 核心の値だけピンクで強調
    #[serde(default)]
    pub highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// 矢印のラベル（必須）。番号は描画時に順番で自動で付く
    pub label: String,
    #[serde(default)]
    pub dashed: bool,
    #[serde(default)]
    pub highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub members: Vec<String>,
    #[serde(default)]
    pub side: Side,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SequenceDiagram {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    /// alt / opt / loop の枠（1 つまで）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<Fragment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub side: Side,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageStyle {
    /// 同期（実線・塗り矢印）
    #[default]
    Sync,
    /// 非同期（実線・開き矢印）
    Async,
    /// 応答（破線）
    Reply,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(default)]
    pub style: MessageStyle,
    #[serde(default)]
    pub highlight: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FragmentKind {
    Alt,
    Opt,
    Loop,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    pub kind: FragmentKind,
    /// 条件（例: 「証明書が有効」）
    pub label: String,
    /// 枠に入る最初のメッセージの番号（0 始まり）
    pub start: usize,
    /// 枠に入る最後のメッセージの番号（0 始まり、含む）
    pub end: usize,
    /// alt の else 側が始まるメッセージの番号
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub else_start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub else_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateDiagram {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub states: Vec<StateNode>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateNode {
    pub id: String,
    pub label: String,
    pub row: u32,
    pub col: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    /// 開始状態
    #[serde(default)]
    pub initial: bool,
    /// 終了状態
    #[serde(default, rename = "final")]
    pub is_final: bool,
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub from: String,
    pub to: String,
    /// イベント（必須）。ラベルは「イベント [ガード] / アクション」で描かれる
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default)]
    pub highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComparisonDiagram {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 比べる対象（例: ["共通鍵暗号", "公開鍵暗号"]）
    pub columns: Vec<String>,
    /// 観点ごとの行
    pub rows: Vec<ComparisonRow>,
    /// 強調するセル
    #[serde(default)]
    pub highlights: Vec<CellRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRow {
    /// 観点（例: 「鍵の数」）
    pub label: String,
    /// columns と同じ数のセル
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CellRef {
    pub row: usize,
    pub col: usize,
}

/// 検査結果。errors があれば保存しない。warnings は保存した上で Claude に返す。
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 図解データの JSON Schema（MCP のプロンプトで Claude に配る）。
pub fn json_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(Diagram)).unwrap_or_default()
}

/// 古い版のデータを現在の版として読めるように読み替える。v1 しかないので今は検査だけ。
pub fn upgrade(value: serde_json::Value) -> std::result::Result<serde_json::Value, String> {
    let version = value.get("version").and_then(|v| v.as_u64());
    match version {
        Some(v) if v as u32 == SCHEMA_VERSION => Ok(value),
        Some(v) => Err(format!("未対応の版です（version={v}、対応は {SCHEMA_VERSION}）")),
        None => Err("version がありません（現在の版は 1）".into()),
    }
}

/// 図解データ 1 つを検査する。
pub fn validate_value(value: &serde_json::Value) -> (Option<Diagram>, Report) {
    let mut report = Report::default();
    let value = match upgrade(value.clone()) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(e);
            return (None, report);
        }
    };
    let diagram: Diagram = match serde_json::from_value(value) {
        Ok(d) => d,
        Err(e) => {
            report.errors.push(format!(
                "形式が不正です: {e}（kind は concept / sequence / state / comparison のいずれか）"
            ));
            return (None, report);
        }
    };
    validate(&diagram, &mut report);
    (Some(diagram), report)
}

/// 複数の図解データを検査する。エラーには「図N:」を付ける。
pub fn validate_all(values: &[serde_json::Value]) -> Report {
    let mut all = Report::default();
    for (i, v) in values.iter().enumerate() {
        let (_, r) = validate_value(v);
        all.errors.extend(r.errors.into_iter().map(|e| format!("図{}: {e}", i + 1)));
        all.warnings.extend(r.warnings.into_iter().map(|w| format!("図{}: {w}", i + 1)));
    }
    all
}

pub fn validate(d: &Diagram, r: &mut Report) {
    match d {
        Diagram::Concept(c) => validate_concept(c, r),
        Diagram::Sequence(s) => validate_sequence(s, r),
        Diagram::State(s) => validate_state(s, r),
        Diagram::Comparison(c) => validate_comparison(c, r),
    }
}

fn check_title(title: &Option<String>, r: &mut Report) {
    if let Some(t) = title {
        check_width("タイトル", t, MAX_TITLE_WIDTH, r);
    }
}

fn check_width(what: &str, text: &str, max: usize, r: &mut Report) {
    let w = display_width(text);
    if w > max {
        r.errors.push(format!(
            "{what}「{text}」が長すぎて枠からはみ出します（幅 {w}、上限 {max}。全角は 2 と数える）。短くしてください"
        ));
    }
}

fn check_label_present(what: &str, text: &str, r: &mut Report) {
    if text.trim().is_empty() {
        r.errors.push(format!("{what}にラベルがありません。すべての矢印にラベルを付けてください"));
    }
}

fn check_count(what: &str, n: usize, max: usize, r: &mut Report) {
    if n == 0 {
        r.errors.push(format!("{what}が 1 つもありません"));
    } else if n > max {
        r.errors.push(format!(
            "{what}が {n} 個あります。1 図の要素は 7±2（最大 {max}）に収め、多い場合は図を分けてください"
        ));
    }
}

fn check_ids<'a>(what: &str, ids: impl Iterator<Item = &'a str>, r: &mut Report) -> HashSet<&'a str> {
    let mut set = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            r.errors.push(format!("{what}の id が空です"));
        } else if !set.insert(id) {
            r.errors.push(format!("{what}の id「{id}」が重複しています"));
        }
    }
    set
}

fn check_ref(what: &str, id: &str, ids: &HashSet<&str>, r: &mut Report) {
    if !ids.contains(id) {
        r.errors.push(format!("{what}が存在しない id「{id}」を参照しています"));
    }
}

fn check_highlights(n: usize, r: &mut Report) {
    if n > MAX_HIGHLIGHTS {
        r.warnings.push(format!(
            "強調（ピンク）が {n} 箇所あります。核心の値だけ（{MAX_HIGHLIGHTS} 箇所まで）にすると伝わりやすくなります"
        ));
    }
}

/// マス目の位置（または上書き座標）から中心座標を求める。アプリ側の配置と同じ計算。
pub fn center(row: u32, col: u32, x: Option<f64>, y: Option<f64>) -> (f64, f64) {
    (
        x.unwrap_or(col as f64 * CELL_W + CELL_W / 2.0),
        y.unwrap_or(row as f64 * CELL_H + CELL_H / 2.0),
    )
}

fn check_grid<'a>(
    what: &str,
    cells: impl Iterator<Item = (&'a str, u32, u32, bool)>,
    r: &mut Report,
) {
    let mut seen: Vec<((u32, u32), &str)> = Vec::new();
    for (id, row, col, overridden) in cells {
        if row > MAX_GRID || col > MAX_GRID {
            r.errors.push(format!("{what}「{id}」の位置（row={row}, col={col}）が大きすぎます（0〜{MAX_GRID}）"));
        }
        if overridden {
            continue;
        }
        if let Some((_, other)) = seen.iter().find(|(c, _)| *c == (row, col)) {
            r.errors.push(format!(
                "{what}「{id}」と「{other}」が同じマス（row={row}, col={col}）に置かれています"
            ));
        } else {
            seen.push(((row, col), id));
        }
    }
}

type Pt = (f64, f64);

fn segments_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    fn orient(p: Pt, q: Pt, r: Pt) -> f64 {
        (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
    }
    let eq = |p: Pt, q: Pt| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6;
    // 端点を共有する線どうしは交差とみなさない
    if eq(a, c) || eq(a, d) || eq(b, c) || eq(b, d) {
        return false;
    }
    let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
    (o1 * o2 < 0.0) && (o3 * o4 < 0.0)
}

/// 直線で結んだときの線の交差を数える（描画も直線＋端の調整なので同じ判定になる）。
fn check_crossings(lines: &[(String, Pt, Pt)], r: &mut Report) {
    for i in 0..lines.len() {
        for j in (i + 1)..lines.len() {
            let (ref la, a1, a2) = lines[i];
            let (ref lb, b1, b2) = lines[j];
            if segments_cross(a1, a2, b1, b2) {
                r.warnings.push(format!(
                    "矢印「{la}」と「{lb}」が交差しています。部品の行・列を入れ替えると読みやすくなります"
                ));
            }
        }
    }
}

fn validate_concept(c: &ConceptDiagram, r: &mut Report) {
    check_title(&c.title, r);
    check_count("部品", c.nodes.len(), MAX_ELEMENTS, r);
    let ids = check_ids("部品", c.nodes.iter().map(|n| n.id.as_str()), r);
    for n in &c.nodes {
        check_width("部品名", &n.label, MAX_LABEL_WIDTH, r);
        if let Some(role) = &n.role {
            check_width("役割", role, MAX_ROLE_WIDTH, r);
        }
        if n.x.is_some() != n.y.is_some() {
            r.errors.push(format!("部品「{}」の座標は x と y を両方指定してください", n.id));
        }
    }
    check_grid(
        "部品",
        c.nodes.iter().map(|n| (n.id.as_str(), n.row, n.col, n.x.is_some() && n.y.is_some())),
        r,
    );
    let mut lines = Vec::new();
    for e in &c.edges {
        check_label_present(&format!("矢印（{} → {}）", e.from, e.to), &e.label, r);
        check_width("矢印のラベル", &e.label, MAX_EDGE_LABEL_WIDTH, r);
        check_ref("矢印", &e.from, &ids, r);
        check_ref("矢印", &e.to, &ids, r);
        if e.from == e.to {
            r.errors.push(format!("矢印が同じ部品「{}」に戻っています", e.from));
        }
        let p = |id: &str| c.nodes.iter().find(|n| n.id == id).map(|n| center(n.row, n.col, n.x, n.y));
        if let (Some(a), Some(b)) = (p(&e.from), p(&e.to)) {
            lines.push((e.label.clone(), a, b));
        }
    }
    check_crossings(&lines, r);
    let gids = check_ids("枠", c.groups.iter().map(|g| g.id.as_str()), r);
    for g in &c.groups {
        if gids.contains(g.id.as_str()) && ids.contains(g.id.as_str()) {
            r.errors.push(format!("枠の id「{}」が部品の id と重複しています", g.id));
        }
        check_width("枠のラベル", &g.label, MAX_LABEL_WIDTH, r);
        if g.members.is_empty() {
            r.errors.push(format!("枠「{}」に部品が入っていません", g.label));
        }
        for m in &g.members {
            check_ref(&format!("枠「{}」", g.label), m, &ids, r);
        }
    }
    check_highlights(
        c.nodes.iter().filter(|n| n.highlight).count() + c.edges.iter().filter(|e| e.highlight).count(),
        r,
    );
}

fn validate_sequence(s: &SequenceDiagram, r: &mut Report) {
    check_title(&s.title, r);
    check_count("登場者", s.participants.len(), MAX_ELEMENTS, r);
    let ids = check_ids("登場者", s.participants.iter().map(|p| p.id.as_str()), r);
    for p in &s.participants {
        check_width("登場者名", &p.label, MAX_LABEL_WIDTH, r);
    }
    check_count("メッセージ", s.messages.len(), MAX_MESSAGES, r);
    for (i, m) in s.messages.iter().enumerate() {
        let what = format!("メッセージ{}（{} → {}）", i + 1, m.from, m.to);
        check_label_present(&what, &m.label, r);
        check_width("メッセージのラベル", &m.label, MAX_EDGE_LABEL_WIDTH + 8, r);
        check_ref(&what, &m.from, &ids, r);
        check_ref(&what, &m.to, &ids, r);
    }
    if let Some(f) = &s.fragment {
        check_width("枠の条件", &f.label, MAX_EDGE_LABEL_WIDTH, r);
        if f.start > f.end || f.end >= s.messages.len() {
            r.errors.push(format!(
                "枠（{:?}）の範囲 start={} end={} がメッセージの番号（0〜{}）に収まっていません",
                f.kind,
                f.start,
                f.end,
                s.messages.len().saturating_sub(1)
            ));
        }
        if let Some(es) = f.else_start {
            if f.kind != FragmentKind::Alt {
                r.errors.push("else_start は alt の枠でだけ使えます".into());
            } else if es <= f.start || es > f.end {
                r.errors.push(format!("else_start={es} は start より後、end 以下にしてください"));
            }
        }
    }
    check_highlights(s.messages.iter().filter(|m| m.highlight).count(), r);
}

fn validate_state(s: &StateDiagram, r: &mut Report) {
    check_title(&s.title, r);
    check_count("状態", s.states.len(), MAX_ELEMENTS, r);
    let ids = check_ids("状態", s.states.iter().map(|n| n.id.as_str()), r);
    for n in &s.states {
        check_width("状態名", &n.label, MAX_LABEL_WIDTH, r);
        if n.x.is_some() != n.y.is_some() {
            r.errors.push(format!("状態「{}」の座標は x と y を両方指定してください", n.id));
        }
    }
    if !s.states.iter().any(|n| n.initial) {
        r.warnings.push("開始状態（initial: true）がありません".into());
    }
    check_grid(
        "状態",
        s.states.iter().map(|n| (n.id.as_str(), n.row, n.col, n.x.is_some() && n.y.is_some())),
        r,
    );
    let mut lines = Vec::new();
    for t in &s.transitions {
        let what = format!("遷移（{} → {}）", t.from, t.to);
        check_label_present(&what, &t.event, r);
        let label = transition_label(t);
        check_width("遷移のラベル", &label, MAX_EDGE_LABEL_WIDTH + 12, r);
        check_ref(&what, &t.from, &ids, r);
        check_ref(&what, &t.to, &ids, r);
        let p = |id: &str| s.states.iter().find(|n| n.id == id).map(|n| center(n.row, n.col, n.x, n.y));
        if t.from != t.to {
            if let (Some(a), Some(b)) = (p(&t.from), p(&t.to)) {
                lines.push((label, a, b));
            }
        }
    }
    check_crossings(&lines, r);
    check_highlights(
        s.states.iter().filter(|n| n.highlight).count() + s.transitions.iter().filter(|t| t.highlight).count(),
        r,
    );
}

/// 遷移のラベル「イベント [ガード] / アクション」。
pub fn transition_label(t: &Transition) -> String {
    let mut s = t.event.clone();
    if let Some(g) = &t.guard {
        s.push_str(&format!(" [{g}]"));
    }
    if let Some(a) = &t.action {
        s.push_str(&format!(" / {a}"));
    }
    s
}

fn validate_comparison(c: &ComparisonDiagram, r: &mut Report) {
    check_title(&c.title, r);
    check_count("比べる対象（columns）", c.columns.len(), 4, r);
    check_count("観点（rows）", c.rows.len(), MAX_ELEMENTS, r);
    for col in &c.columns {
        check_width("列見出し", col, MAX_LABEL_WIDTH, r);
    }
    for (i, row) in c.rows.iter().enumerate() {
        check_width("観点", &row.label, MAX_LABEL_WIDTH, r);
        if row.cells.len() != c.columns.len() {
            r.errors.push(format!(
                "行{}「{}」のセルが {} 個です（columns と同じ {} 個にしてください）",
                i + 1,
                row.label,
                row.cells.len(),
                c.columns.len()
            ));
        }
        for cell in &row.cells {
            check_width("セル", cell, MAX_CELL_WIDTH, r);
        }
    }
    for h in &c.highlights {
        if h.row >= c.rows.len() || h.col >= c.columns.len() {
            r.errors.push(format!("強調セル（row={}, col={}）が表の外です", h.row, h.col));
        }
    }
    check_highlights(c.highlights.len(), r);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn concept_ok() -> serde_json::Value {
        json!({
            "version": 1, "kind": "concept", "title": "WAF の位置",
            "nodes": [
                {"id": "user", "label": "利用者", "side": "external", "row": 0, "col": 0},
                {"id": "waf", "label": "WAF", "role": "HTTP を検査", "row": 0, "col": 1, "highlight": true},
                {"id": "app", "label": "Web アプリ", "row": 0, "col": 2}
            ],
            "edges": [
                {"from": "user", "to": "waf", "label": "HTTP リクエスト"},
                {"from": "waf", "to": "app", "label": "正常な通信だけ通す"}
            ],
            "groups": [{"id": "dc", "label": "自社", "members": ["waf", "app"]}]
        })
    }

    #[test]
    fn valid_concept_passes() {
        let r = validate_all(&[concept_ok()]);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn missing_label_and_unknown_ref_fail() {
        let mut v = concept_ok();
        v["edges"][0]["label"] = json!("");
        v["edges"][1]["to"] = json!("db");
        let r = validate_all(&[v]);
        assert!(r.errors.iter().any(|e| e.contains("ラベルがありません")));
        assert!(r.errors.iter().any(|e| e.contains("「db」")));
        assert!(r.errors.iter().all(|e| e.starts_with("図1:")));
    }

    #[test]
    fn too_many_elements_fail() {
        let nodes: Vec<_> = (0..10)
            .map(|i| json!({"id": format!("n{i}"), "label": "x", "row": i / 5, "col": i % 5}))
            .collect();
        let v = json!({"version": 1, "kind": "concept", "nodes": nodes});
        let r = validate_all(&[v]);
        assert!(r.errors.iter().any(|e| e.contains("10 個")));
    }

    #[test]
    fn overflow_and_same_cell_fail() {
        let mut v = concept_ok();
        v["nodes"][0]["label"] = json!("とても長い部品名でマスからはみ出すラベル");
        v["nodes"][2]["col"] = json!(1);
        let r = validate_all(&[v]);
        assert!(r.errors.iter().any(|e| e.contains("はみ出します")));
        assert!(r.errors.iter().any(|e| e.contains("同じマス")));
    }

    #[test]
    fn crossing_is_warning() {
        let v = json!({
            "version": 1, "kind": "concept",
            "nodes": [
                {"id": "a", "label": "A", "row": 0, "col": 0},
                {"id": "b", "label": "B", "row": 1, "col": 1},
                {"id": "c", "label": "C", "row": 0, "col": 1},
                {"id": "d", "label": "D", "row": 1, "col": 0}
            ],
            "edges": [
                {"from": "a", "to": "b", "label": "1"},
                {"from": "c", "to": "d", "label": "2"}
            ]
        });
        let r = validate_all(&[v]);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].contains("交差"));
    }

    #[test]
    fn unknown_fields_and_versions_rejected() {
        let mut v = concept_ok();
        v["nodes"][0]["color"] = json!("#ff0000");
        assert!(!validate_all(&[v]).errors.is_empty());
        let mut v = concept_ok();
        v["version"] = json!(2);
        assert!(validate_all(&[v]).errors[0].contains("未対応の版"));
        let v = json!({"version": 1, "kind": "svg", "svg": "<svg/>"});
        assert!(!validate_all(&[v]).errors.is_empty());
    }

    #[test]
    fn sequence_rules() {
        let v = json!({
            "version": 1, "kind": "sequence", "title": "TLS 1.3 ハンドシェイク",
            "participants": [
                {"id": "c", "label": "クライアント"},
                {"id": "s", "label": "サーバー", "side": "external"}
            ],
            "messages": [
                {"from": "c", "to": "s", "label": "ClientHello"},
                {"from": "s", "to": "c", "label": "ServerHello + 証明書", "style": "reply"},
                {"from": "c", "to": "s", "label": "Finished"}
            ],
            "fragment": {"kind": "alt", "label": "証明書が有効", "start": 1, "end": 2, "else_start": 2}
        });
        let r = validate_all(std::slice::from_ref(&v));
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let mut bad = v;
        bad["fragment"]["end"] = json!(5);
        assert!(validate_all(&[bad]).errors.iter().any(|e| e.contains("範囲")));
    }

    #[test]
    fn state_rules() {
        let v = json!({
            "version": 1, "kind": "state",
            "states": [
                {"id": "closed", "label": "CLOSED", "row": 0, "col": 0, "initial": true},
                {"id": "syn", "label": "SYN_SENT", "row": 0, "col": 1},
                {"id": "est", "label": "ESTABLISHED", "row": 0, "col": 2, "highlight": true}
            ],
            "transitions": [
                {"from": "closed", "to": "syn", "event": "connect", "action": "SYN 送信"},
                {"from": "syn", "to": "est", "event": "SYN+ACK 受信", "action": "ACK 送信"},
                {"from": "est", "to": "est", "event": "データ"}
            ]
        });
        let r = validate_all(&[v]);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    }

    #[test]
    fn comparison_rules() {
        let v = json!({
            "version": 1, "kind": "comparison",
            "columns": ["共通鍵暗号", "公開鍵暗号"],
            "rows": [
                {"label": "鍵の数", "cells": ["1 つを共有", "公開鍵と秘密鍵の 2 つ"]},
                {"label": "速さ", "cells": ["速い"]}
            ],
            "highlights": [{"row": 0, "col": 1}, {"row": 9, "col": 0}]
        });
        let r = validate_all(&[v]);
        assert!(r.errors.iter().any(|e| e.contains("セルが 1 個")));
        assert!(r.errors.iter().any(|e| e.contains("表の外")));
    }

    #[test]
    fn schema_is_generated() {
        let s = json_schema();
        assert!(s.to_string().contains("concept"));
    }
}
