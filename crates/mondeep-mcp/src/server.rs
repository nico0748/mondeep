//! MCP ツール（8 個）とプロンプト（2 個）。削除系の操作は公開しない。
use std::sync::{Arc, Mutex};

use chrono::Local;
use mondeep_core::model::{Format, SessionFormat, Source, Verdict};
use mondeep_core::store::{RecordAttempt, RecordQuestion, TermInput};
use mondeep_core::{Error, Store};
use rmcp::handler::server::router::prompt::PromptRouter;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{prompt, prompt_handler, prompt_router, schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use serde::Deserialize;
use serde_json::json;

use crate::guides;
use crate::Config;

#[derive(Clone)]
pub struct MondeepServer {
    store: Arc<Mutex<Store>>,
    cfg: Arc<Config>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartSessionParams {
    /// 出題形式: meaning_to_term（意味→用語）/ term_to_meaning（用語→意味＋深掘り）/ mixed（両方）
    pub format: SessionFormat,
    /// 会話のテーマ（例: 「TLS 周りを意味→用語で 10 問」）
    pub theme: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct QueueParams {
    /// 取得する件数（既定 10）
    pub limit: Option<i64>,
    /// 分野タグで絞り込む（いずれかに一致）
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TermHistoryParams {
    /// 用語（見出し語または別表記）
    pub term: Option<String>,
    /// 用語 ID（分かっていれば）
    pub term_id: Option<i64>,
    /// 返す件数（既定 30）
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecordQuestionParams {
    pub session_id: i64,
    /// 問題文（チャットで出した文のまま）
    pub prompt: String,
    /// 形式。単一形式のセッションと深掘りの子問題では省略できる
    pub format: Option<Format>,
    /// 用語の見出し語。深掘りの子問題では省略すると親の用語を引き継ぐ
    pub term: Option<String>,
    /// 別表記（略語・英語・カタカナなど）。正解扱いにする表記ゆれ
    pub aliases: Option<Vec<String>>,
    /// 分野タグ（Web / 暗号 / ネットワーク / 面接 など）
    pub tags: Option<Vec<String>>,
    /// 「近い用語があります」と返ったときに選んだ既存の用語 ID
    pub term_id: Option<i64>,
    /// 近い用語があっても新しい用語として登録する
    pub create_new_term: Option<bool>,
    /// 深掘りの親問題 ID（最初の問いでは省略）
    pub parent_question_id: Option<i64>,
    /// 模範解答
    pub model_answer: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecordAttemptParams {
    pub question_id: i64,
    /// Nico さんの回答（チャットのまま）
    pub answer: String,
    /// 判定: correct / partial / incorrect
    pub verdict: Verdict,
    /// 判定理由（含まれていた要素・足りなかった要素）
    pub reason: String,
    /// 解説（Markdown。生の HTML は表示されない）
    pub explanation_markdown: Option<String>,
    /// 図解データ（diagram_guide の形式、0 個以上）
    pub diagrams: Option<Vec<serde_json::Value>>,
    /// 出典（Obsidian ノート名、ナレッジ DB の項目、URL）
    pub sources: Option<Vec<Source>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EndSessionParams {
    pub session_id: i64,
    /// まとめ（正答率の所感、弱点、次に復習すべき観点）
    pub summary: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// キーワード（用語・問題文・解説を部分一致で検索）
    pub keyword: String,
    /// 件数（既定 20）
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StudyGuideParams {
    /// 形式: meaning_to_term / term_to_meaning（省略可）
    pub format: Option<String>,
}

fn ok(v: serde_json::Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(&v).unwrap_or_default(),
    )]))
}

/// ツールの失敗は Claude が読んで直せるよう、プロトコルエラーではなくツールのエラー結果で返す。
fn fail(e: Error) -> Result<CallToolResult, McpError> {
    let body = match &e {
        Error::AmbiguousTerm { candidates, .. } => json!({ "error": e.to_string(), "candidates": candidates }),
        Error::Diagram(errors) => json!({ "error": "図解データが規約に違反しています。直して保存し直してください", "violations": errors }),
        _ => json!({ "error": e.to_string() }),
    };
    Ok(CallToolResult::error(vec![ContentBlock::text(serde_json::to_string_pretty(&body).unwrap_or_default())]))
}

macro_rules! tryf {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return fail(e.into()),
        }
    };
}

impl MondeepServer {
    pub fn new(store: Store, cfg: Config) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            cfg: Arc::new(cfg),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[tool_router]
impl MondeepServer {
    #[tool(description = "学習セッションを開始し、セッション ID を返す。1 回の会話につき 1 つ。")]
    pub async fn start_session(&self, Parameters(p): Parameters<StartSessionParams>) -> Result<CallToolResult, McpError> {
        let s = tryf!(self.store().start_session(p.format, &p.theme));
        ok(json!({ "session_id": s.id, "started_at": s.started_at }))
    }

    #[tool(description = "今日復習すべき用語（次回復習日が来たもの・要復習の印があるもの）を、過去の正誤の要約つきで返す。")]
    pub async fn get_review_queue(&self, Parameters(p): Parameters<QueueParams>) -> Result<CallToolResult, McpError> {
        let today = Local::now().date_naive();
        let items = tryf!(self.store().review_queue(p.limit.unwrap_or(10), &p.tags.unwrap_or_default(), today));
        ok(json!({ "today": today.to_string(), "count": items.len(), "items": items }))
    }

    #[tool(description = "用語の過去の出題・回答・判定を返す。同じ問いの繰り返しを避けるため、出題前に確かめる。Nico さんがアプリで直した判定（original_verdict が元の判定）とメモ（note）も含む。")]
    pub async fn get_term_history(&self, Parameters(p): Parameters<TermHistoryParams>) -> Result<CallToolResult, McpError> {
        let store = self.store();
        let term_id = match (p.term_id, p.term.as_deref()) {
            (Some(id), _) => id,
            (None, Some(name)) => match tryf!(store.find_term(name)) {
                Some(t) => t.id,
                None => {
                    let candidates = tryf!(store.similar_terms(&[name.to_string()]));
                    return ok(json!({ "found": false, "message": format!("「{name}」はまだ出題されていません"), "similar_terms": candidates }));
                }
            },
            (None, None) => return fail(Error::Invalid("term か term_id を指定してください".into())),
        };
        let h = tryf!(store.term_history(term_id, p.limit.unwrap_or(30)));
        ok(json!({ "found": true, "history": h }))
    }

    #[tool(description = "問題を保存し、問題 ID を返す。出題する前に呼ぶ。深掘りの追加質問は parent_question_id を付ける（深さは自動で記録）。近い既存の用語があると候補が返るので、term_id か create_new_term を付けて呼び直す。")]
    pub async fn record_question(&self, Parameters(p): Parameters<RecordQuestionParams>) -> Result<CallToolResult, McpError> {
        let term = if p.term.is_some() || p.term_id.is_some() {
            Some(TermInput {
                headword: p.term.unwrap_or_default(),
                aliases: p.aliases.unwrap_or_default(),
                tags: p.tags.unwrap_or_default(),
                term_id: p.term_id,
                create_new: p.create_new_term.unwrap_or(false),
            })
        } else {
            None
        };
        let store = self.store();
        let q = tryf!(store.record_question(&RecordQuestion {
            session_id: p.session_id,
            format: p.format,
            prompt: p.prompt,
            model_answer: p.model_answer,
            term,
            parent_id: p.parent_question_id,
        }));
        let t = tryf!(store.get_term(q.term_id));
        ok(json!({
            "question_id": q.id, "depth": q.depth, "format": q.format,
            "term": { "term_id": t.id, "headword": t.headword, "aliases": t.aliases, "tags": t.tags }
        }))
    }

    #[tool(description = "回答・判定（correct / partial / incorrect）・判定理由・解説 Markdown・図解データ・出典を保存し、更新後の次回復習日を返す。図解データは規約で検査され、違反があると保存せず理由を返す。")]
    pub async fn record_attempt(&self, Parameters(p): Parameters<RecordAttemptParams>) -> Result<CallToolResult, McpError> {
        let today = Local::now().date_naive();
        let r = tryf!(self.store().record_attempt(
            &RecordAttempt {
                question_id: p.question_id,
                answer: p.answer,
                verdict: p.verdict,
                reason: p.reason,
                explanation_md: p.explanation_markdown,
                diagrams: p.diagrams.unwrap_or_default(),
                sources: p.sources.unwrap_or_default(),
            },
            today,
        ));
        ok(json!({
            "attempt_id": r.attempt_id,
            "next_review_date": r.review.due_date,
            "interval_days": r.review.interval_days,
            "streak": r.review.streak,
            "diagram_warnings": r.warnings,
        }))
    }

    #[tool(description = "セッションを終了し、まとめ文を保存して正答率（部分正解は 0.5 点）を返す。")]
    pub async fn end_session(&self, Parameters(p): Parameters<EndSessionParams>) -> Result<CallToolResult, McpError> {
        let r = tryf!(self.store().end_session(p.session_id, &p.summary));
        ok(serde_json::to_value(r).unwrap_or_default())
    }

    #[tool(description = "苦手な用語（不正解が多い、深掘りが浅い段で止まる、要復習の印がある）を理由つきで返す。")]
    pub async fn list_weak_terms(&self, Parameters(p): Parameters<QueueParams>) -> Result<CallToolResult, McpError> {
        let items = tryf!(self.store().weak_terms(p.limit.unwrap_or(10), &p.tags.unwrap_or_default()));
        ok(json!({ "count": items.len(), "items": items }))
    }

    #[tool(description = "過去の用語・問題文・解説をキーワードで全文検索する。")]
    pub async fn search(&self, Parameters(p): Parameters<SearchParams>) -> Result<CallToolResult, McpError> {
        let hits = tryf!(self.store().search(&p.keyword, p.limit.unwrap_or(20)));
        ok(json!({ "count": hits.len(), "hits": hits }))
    }
}

#[prompt_router]
impl MondeepServer {
    #[prompt(name = "study_guide", description = "出題の進め方・判定基準・解説の書き方のガイド。学習を始める前に読む。")]
    pub async fn study_guide(&self, Parameters(p): Parameters<StudyGuideParams>) -> Vec<PromptMessage> {
        vec![PromptMessage::new_text(Role::User, guides::study_guide(&self.cfg, p.format.as_deref()))]
    }

    #[prompt(name = "diagram_guide", description = "図解データ（概念図・シーケンス図・状態遷移図・比較表）の書き方と規約、JSON Schema。")]
    pub async fn diagram_guide(&self) -> Vec<PromptMessage> {
        vec![PromptMessage::new_text(Role::User, guides::diagram_guide())]
    }
}

#[tool_handler(router = self.tool_router)]
#[prompt_handler(router = self.prompt_router)]
impl ServerHandler for MondeepServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_prompts().build())
            .with_server_info(Implementation::new("mondeep", env!("CARGO_PKG_VERSION")))
            .with_instructions(guides::SERVER_INSTRUCTIONS)
    }
}
