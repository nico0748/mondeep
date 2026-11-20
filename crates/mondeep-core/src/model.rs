//! アプリと MCP サーバーが共有するデータ型。
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 出題形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// 意味→用語（意味を問題文として出し、用語を答えさせる）
    MeaningToTerm,
    /// 用語→意味＋深掘り（用語を出し意味を答えさせ、追加質問を続ける）
    TermToMeaning,
}

impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::MeaningToTerm => "meaning_to_term",
            Format::TermToMeaning => "term_to_meaning",
        }
    }
    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "meaning_to_term" => Ok(Format::MeaningToTerm),
            "term_to_meaning" => Ok(Format::TermToMeaning),
            other => Err(crate::Error::Invalid(format!("不明な形式: {other}"))),
        }
    }
}

/// セッションの形式。1 回の会話で両方を混ぜることもあるので mixed を許す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionFormat {
    MeaningToTerm,
    TermToMeaning,
    Mixed,
}

impl SessionFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionFormat::MeaningToTerm => "meaning_to_term",
            SessionFormat::TermToMeaning => "term_to_meaning",
            SessionFormat::Mixed => "mixed",
        }
    }
    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "meaning_to_term" => Ok(SessionFormat::MeaningToTerm),
            "term_to_meaning" => Ok(SessionFormat::TermToMeaning),
            "mixed" => Ok(SessionFormat::Mixed),
            other => Err(crate::Error::Invalid(format!("不明なセッション形式: {other}"))),
        }
    }
    pub fn single(&self) -> Option<Format> {
        match self {
            SessionFormat::MeaningToTerm => Some(Format::MeaningToTerm),
            SessionFormat::TermToMeaning => Some(Format::TermToMeaning),
            SessionFormat::Mixed => None,
        }
    }
}

/// 正誤判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Correct,
    Partial,
    Incorrect,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Correct => "correct",
            Verdict::Partial => "partial",
            Verdict::Incorrect => "incorrect",
        }
    }
    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "correct" => Ok(Verdict::Correct),
            "partial" => Ok(Verdict::Partial),
            "incorrect" => Ok(Verdict::Incorrect),
            other => Err(crate::Error::Invalid(format!("不明な判定: {other}"))),
        }
    }
    /// 正答率の計算に使う点数（部分正解は 0.5）。
    pub fn score(&self) -> f64 {
        match self {
            Verdict::Correct => 1.0,
            Verdict::Partial => 0.5,
            Verdict::Incorrect => 0.0,
        }
    }
}

/// 解説の出典の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Obsidian,
    KnowledgeDb,
    Web,
    Other,
}

/// 解説の出典（Obsidian ノート名、ナレッジ DB の項目、URL）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Source {
    pub kind: SourceKind,
    /// ノート名・項目名・ページタイトル
    pub name: String,
    /// Web の場合の URL（http/https のみ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Term {
    pub id: i64,
    pub headword: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub needs_review: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: i64,
    pub format: SessionFormat,
    pub theme: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: Option<String>,
    pub accuracy: Option<f64>,
}

/// 履歴一覧の 1 行（セッション＋集計）。
#[derive(Debug, Clone, Serialize)]
pub struct SessionListItem {
    #[serde(flatten)]
    pub session: Session,
    pub question_count: i64,
    pub correct: i64,
    pub partial: i64,
    pub incorrect: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Question {
    pub id: i64,
    pub session_id: i64,
    pub term_id: i64,
    pub format: Format,
    pub prompt: String,
    pub model_answer: Option<String>,
    pub parent_id: Option<i64>,
    pub depth: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub id: i64,
    pub question_id: i64,
    pub answer: String,
    pub verdict: Verdict,
    pub reason: String,
    pub answered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Explanation {
    pub id: i64,
    pub question_id: i64,
    pub attempt_id: Option<i64>,
    pub body_md: String,
    /// 図解データ（8 章の形式）。検査済みの JSON をそのまま渡す。
    pub diagrams: Vec<serde_json::Value>,
    pub sources: Vec<Source>,
    pub created_at: String,
}

/// 一覧・木構造で使う、問題とその最新の判定。
#[derive(Debug, Clone, Serialize)]
pub struct QuestionSummary {
    #[serde(flatten)]
    pub question: Question,
    pub headword: String,
    pub latest_verdict: Option<Verdict>,
    pub attempt_count: i64,
}

/// 問題詳細画面のデータ。
#[derive(Debug, Clone, Serialize)]
pub struct QuestionDetail {
    pub question: Question,
    pub term: Term,
    pub session: Session,
    pub attempts: Vec<Attempt>,
    pub explanation: Option<Explanation>,
    /// 同じ深掘りの木に属する全問題（根から）。
    pub tree: Vec<QuestionSummary>,
    pub root_id: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionDetail {
    pub session: Session,
    pub questions: Vec<QuestionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewState {
    pub term_id: i64,
    pub format: Format,
    pub due_date: String,
    pub interval_days: i64,
    pub ease: f64,
    pub streak: i64,
    pub last_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct VerdictCounts {
    pub correct: i64,
    pub partial: i64,
    pub incorrect: i64,
}

impl VerdictCounts {
    pub fn total(&self) -> i64 {
        self.correct + self.partial + self.incorrect
    }
    pub fn accuracy(&self) -> Option<f64> {
        let t = self.total();
        if t == 0 {
            None
        } else {
            Some((self.correct as f64 + 0.5 * self.partial as f64) / t as f64)
        }
    }
    pub fn add(&mut self, v: Verdict) {
        match v {
            Verdict::Correct => self.correct += 1,
            Verdict::Partial => self.partial += 1,
            Verdict::Incorrect => self.incorrect += 1,
        }
    }
}

/// 用語一覧の 1 行。
#[derive(Debug, Clone, Serialize)]
pub struct TermListItem {
    #[serde(flatten)]
    pub term: Term,
    pub counts: VerdictCounts,
    pub accuracy: Option<f64>,
    pub last_asked_at: Option<String>,
    pub next_due: Option<String>,
}

/// 用語カード。
#[derive(Debug, Clone, Serialize)]
pub struct TermCard {
    pub term: Term,
    pub counts: VerdictCounts,
    pub accuracy: Option<f64>,
    pub review_states: Vec<ReviewState>,
    pub questions: Vec<QuestionSummary>,
    pub latest_explanation: Option<Explanation>,
    /// 深掘り形式で正解できた最も深い段（なければ None）。
    pub max_correct_depth: Option<i64>,
}

/// 復習キューの 1 件。
#[derive(Debug, Clone, Serialize)]
pub struct ReviewItem {
    pub term_id: i64,
    pub headword: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    /// 復習する形式。要復習印だけで復習状態がない場合は None。
    pub format: Option<Format>,
    pub due_date: Option<String>,
    pub needs_review: bool,
    pub counts: VerdictCounts,
    pub last_verdict: Option<Verdict>,
    pub summary: String,
}

/// 苦手用語の 1 件。
#[derive(Debug, Clone, Serialize)]
pub struct WeakTerm {
    pub term_id: i64,
    pub headword: String,
    pub tags: Vec<String>,
    pub needs_review: bool,
    pub counts: VerdictCounts,
    pub accuracy: Option<f64>,
    /// 深掘りで正解できた最も深い段の平均（深掘りの記録がなければ None）
    pub avg_max_correct_depth: Option<f64>,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// 検索結果の 1 件。
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    /// term / question / explanation
    pub kind: String,
    pub term_id: i64,
    pub headword: String,
    pub question_id: Option<i64>,
    pub snippet: String,
}

/// 履歴の絞り込み条件。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HistoryFilter {
    pub format: Option<Format>,
    pub tag: Option<String>,
    pub verdict: Option<Verdict>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
