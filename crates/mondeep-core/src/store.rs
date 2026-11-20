//! 学習 DB への読み書き。アプリと MCP サーバーの両方がこの `Store` を使う。
//!
//! すべての SQL はパラメータ化クエリで書く（文字列連結で値を埋め込まない）。
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::diagram;
use crate::error::{Error, TermCandidate};
use crate::model::*;
use crate::normalize::{is_similar, term_key};
use crate::srs::{self, Schedule};
use crate::{db, Result};

pub struct Store {
    conn: Connection,
    path: Option<PathBuf>,
}

/// 問題を保存するときの用語の指定。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TermInput {
    pub headword: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 候補から選んだ既存の用語 ID
    pub term_id: Option<i64>,
    /// 近い用語があっても新しい用語として登録する
    #[serde(default)]
    pub create_new: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RecordQuestion {
    pub session_id: i64,
    pub format: Option<Format>,
    pub prompt: String,
    pub model_answer: Option<String>,
    /// 深掘りの子問題では省略すると親の用語を引き継ぐ
    pub term: Option<TermInput>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RecordAttempt {
    pub question_id: i64,
    pub answer: String,
    pub verdict: Verdict,
    pub reason: String,
    pub explanation_md: Option<String>,
    pub diagrams: Vec<serde_json::Value>,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttemptResult {
    pub attempt_id: i64,
    pub explanation_id: Option<i64>,
    pub review: ReviewState,
    /// 図解データの規約の警告（保存はした）
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionEnd {
    pub session_id: i64,
    pub accuracy: Option<f64>,
    pub counts: VerdictCounts,
    pub question_count: i64,
}

/// 用語の過去の出題 1 件（同じ問いの繰り返しを避けるため Claude に返す）。
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub question_id: i64,
    pub session_id: i64,
    pub session_theme: String,
    pub format: Format,
    pub depth: i64,
    pub parent_id: Option<i64>,
    pub prompt: String,
    pub asked_at: String,
    pub latest_answer: Option<String>,
    pub latest_verdict: Option<Verdict>,
    pub latest_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TermHistory {
    pub term: Term,
    pub counts: VerdictCounts,
    pub review_states: Vec<ReviewState>,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryGroup {
    pub session: SessionListItem,
    pub questions: Vec<QuestionSummary>,
}

/// 微調整で動かせる最大量（px）。
const MAX_ADJUST: f64 = 2000.0;

/// review_states の 1 行（due_date, interval_days, ease, streak, tree_root_id, tree_prev）。
type StoredReview = (String, i64, f64, i64, Option<i64>, Option<String>);

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn date_str(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| Error::Invalid(format!("日付 {s}: {e}")))
}

fn json_vec(s: String) -> Vec<String> {
    serde_json::from_str(&s).unwrap_or_default()
}

fn clean_list(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in items {
        let t = i.trim();
        if !t.is_empty() && !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    }
    out
}

fn conv_err(e: Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e.to_string())))
}

fn term_from_row(r: &Row) -> rusqlite::Result<Term> {
    Ok(Term {
        id: r.get("id")?,
        headword: r.get("headword")?,
        aliases: json_vec(r.get("aliases")?),
        tags: json_vec(r.get("tags")?),
        needs_review: r.get::<_, i64>("needs_review")? != 0,
        created_at: r.get("created_at")?,
    })
}

fn session_from_row(r: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get("id")?,
        format: SessionFormat::parse(&r.get::<_, String>("format")?).map_err(conv_err)?,
        theme: r.get("theme")?,
        started_at: r.get("started_at")?,
        ended_at: r.get("ended_at")?,
        summary: r.get("summary")?,
        accuracy: r.get("accuracy")?,
    })
}

fn question_from_row(r: &Row) -> rusqlite::Result<Question> {
    Ok(Question {
        id: r.get("id")?,
        session_id: r.get("session_id")?,
        term_id: r.get("term_id")?,
        format: Format::parse(&r.get::<_, String>("format")?).map_err(conv_err)?,
        prompt: r.get("prompt")?,
        model_answer: r.get("model_answer")?,
        parent_id: r.get("parent_id")?,
        depth: r.get("depth")?,
        created_at: r.get("created_at")?,
    })
}

fn opt_verdict(s: Option<String>) -> rusqlite::Result<Option<Verdict>> {
    s.map(|s| Verdict::parse(&s).map_err(conv_err)).transpose()
}

fn summary_from_row(r: &Row) -> rusqlite::Result<QuestionSummary> {
    Ok(QuestionSummary {
        question: question_from_row(r)?,
        headword: r.get("headword")?,
        latest_verdict: opt_verdict(r.get("latest_verdict")?)?,
        attempt_count: r.get("attempt_count")?,
    })
}

fn attempt_from_row(r: &Row) -> rusqlite::Result<Attempt> {
    Ok(Attempt {
        id: r.get("id")?,
        question_id: r.get("question_id")?,
        answer: r.get("answer")?,
        verdict: Verdict::parse(&r.get::<_, String>("verdict")?).map_err(conv_err)?,
        reason: r.get("reason")?,
        answered_at: r.get("answered_at")?,
    })
}

fn explanation_from_row(r: &Row) -> rusqlite::Result<Explanation> {
    Ok(Explanation {
        id: r.get("id")?,
        question_id: r.get("question_id")?,
        attempt_id: r.get("attempt_id")?,
        body_md: r.get("body_md")?,
        diagrams: serde_json::from_str(&r.get::<_, String>("diagrams")?).unwrap_or_default(),
        sources: serde_json::from_str(&r.get::<_, String>("sources")?).unwrap_or_default(),
        created_at: r.get("created_at")?,
    })
}

fn review_from_row(r: &Row) -> rusqlite::Result<ReviewState> {
    Ok(ReviewState {
        term_id: r.get("term_id")?,
        format: Format::parse(&r.get::<_, String>("format")?).map_err(conv_err)?,
        due_date: r.get("due_date")?,
        interval_days: r.get("interval_days")?,
        ease: r.get("ease")?,
        streak: r.get("streak")?,
        last_reviewed_at: r.get("last_reviewed_at")?,
    })
}

/// 問題一覧用の SELECT（最新の判定と回答数つき）。
const SUMMARY_SELECT: &str = "
    SELECT q.*, t.headword,
        (SELECT a.verdict FROM attempts a WHERE a.question_id = q.id ORDER BY a.id DESC LIMIT 1) AS latest_verdict,
        (SELECT count(*) FROM attempts a WHERE a.question_id = q.id) AS attempt_count
    FROM questions q JOIN terms t ON t.id = q.term_id";

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Store { conn: db::open(path)?, path: Some(path.to_path_buf()) })
    }

    pub fn open_in_memory() -> Result<Self> {
        Ok(Store { conn: db::open_in_memory()?, path: None })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn data_version(&self) -> Result<i64> {
        db::data_version(&self.conn)
    }

    // ------------------------------------------------------------------ 用語

    pub fn get_term(&self, id: i64) -> Result<Term> {
        self.conn
            .query_row("SELECT * FROM terms WHERE id = ?1", [id], term_from_row)
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("用語 ID {id}")))
    }

    /// 見出し語または別表記が完全に一致する用語を探す。
    pub fn find_term(&self, name: &str) -> Result<Option<Term>> {
        let key = term_key(name);
        let id: Option<i64> = self
            .conn
            .query_row("SELECT term_id FROM term_keys WHERE key = ?1", [&key], |r| r.get(0))
            .optional()?;
        id.map(|id| self.get_term(id)).transpose()
    }

    /// 表記が近い既存の用語（同一判定で迷う候補）。
    pub fn similar_terms(&self, names: &[String]) -> Result<Vec<TermCandidate>> {
        let keys: Vec<String> = names.iter().map(|n| term_key(n)).filter(|k| !k.is_empty()).collect();
        let mut stmt = self.conn.prepare("SELECT key, term_id FROM term_keys")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut ids = BTreeSet::new();
        for row in rows {
            let (k, id) = row?;
            if keys.iter().any(|q| is_similar(q, &k)) {
                ids.insert(id);
            }
        }
        ids.into_iter()
            .map(|id| {
                self.get_term(id).map(|t| TermCandidate { term_id: t.id, headword: t.headword, aliases: t.aliases })
            })
            .collect()
    }

    /// 用語の同一判定。見出し語と別表記で照合し、迷う場合は `Error::AmbiguousTerm` で候補を返す。
    pub fn resolve_term(&self, input: &TermInput) -> Result<Term> {
        let headword = input.headword.trim();
        if input.term_id.is_none() && headword.is_empty() {
            return Err(Error::Invalid("用語（headword）が空です".into()));
        }
        let mut names = vec![headword.to_string()];
        names.extend(clean_list(&input.aliases));
        names.retain(|n| !term_key(n).is_empty());

        if let Some(id) = input.term_id {
            let t = self.get_term(id)?;
            return self.merge_into_term(t, &names, &input.tags);
        }

        // 完全一致
        let mut matched = BTreeSet::new();
        for n in &names {
            if let Some(id) = self
                .conn
                .query_row("SELECT term_id FROM term_keys WHERE key = ?1", [term_key(n)], |r| r.get::<_, i64>(0))
                .optional()?
            {
                matched.insert(id);
            }
        }
        match matched.len() {
            1 => {
                let t = self.get_term(*matched.iter().next().unwrap())?;
                return self.merge_into_term(t, &names, &input.tags);
            }
            n if n > 1 => {
                let candidates = matched
                    .into_iter()
                    .map(|id| self.get_term(id).map(|t| TermCandidate { term_id: t.id, headword: t.headword, aliases: t.aliases }))
                    .collect::<Result<Vec<_>>>()?;
                return Err(Error::AmbiguousTerm { input: headword.to_string(), candidates });
            }
            _ => {}
        }

        if !input.create_new {
            let candidates = self.similar_terms(&names)?;
            if !candidates.is_empty() {
                return Err(Error::AmbiguousTerm { input: headword.to_string(), candidates });
            }
        }
        self.create_term(headword, &names[1..], &input.tags)
    }

    fn create_term(&self, headword: &str, aliases: &[String], tags: &[String]) -> Result<Term> {
        let now = now_iso();
        let aliases: Vec<String> = clean_list(aliases)
            .into_iter()
            .filter(|a| term_key(a) != term_key(headword))
            .collect();
        let tags = clean_list(tags);
        self.conn.execute(
            "INSERT INTO terms(headword, aliases, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![headword, serde_json::to_string(&aliases)?, serde_json::to_string(&tags)?, now],
        )?;
        let id = self.conn.last_insert_rowid();
        for n in std::iter::once(headword.to_string()).chain(aliases.iter().cloned()) {
            self.conn.execute(
                "INSERT OR IGNORE INTO term_keys(key, term_id) VALUES (?1, ?2)",
                params![term_key(&n), id],
            )?;
        }
        self.get_term(id)
    }

    /// 既存の用語に新しい別表記とタグを足す。他の用語のキーと衝突する別表記は足さない。
    fn merge_into_term(&self, t: Term, names: &[String], tags: &[String]) -> Result<Term> {
        let mut aliases = t.aliases.clone();
        let mut changed = false;
        for n in names {
            let k = term_key(n);
            if k.is_empty() || k == term_key(&t.headword) || aliases.iter().any(|a| term_key(a) == k) {
                continue;
            }
            let owner: Option<i64> = self
                .conn
                .query_row("SELECT term_id FROM term_keys WHERE key = ?1", [&k], |r| r.get(0))
                .optional()?;
            if owner.is_some_and(|o| o != t.id) {
                continue;
            }
            self.conn.execute("INSERT OR IGNORE INTO term_keys(key, term_id) VALUES (?1, ?2)", params![k, t.id])?;
            aliases.push(n.trim().to_string());
            changed = true;
        }
        let mut new_tags = t.tags.clone();
        for tag in clean_list(tags) {
            if !new_tags.contains(&tag) {
                new_tags.push(tag);
                changed = true;
            }
        }
        if changed {
            self.conn.execute(
                "UPDATE terms SET aliases = ?1, tags = ?2, updated_at = ?3 WHERE id = ?4",
                params![serde_json::to_string(&aliases)?, serde_json::to_string(&new_tags)?, now_iso(), t.id],
            )?;
        }
        self.get_term(t.id)
    }

    /// 要復習の印を付け外しする（アプリから）。
    pub fn set_needs_review(&self, term_id: i64, on: bool) -> Result<Term> {
        let n = self.conn.execute(
            "UPDATE terms SET needs_review = ?1, updated_at = ?2 WHERE id = ?3",
            params![on as i64, now_iso(), term_id],
        )?;
        if n == 0 {
            return Err(Error::NotFound(format!("用語 ID {term_id}")));
        }
        self.get_term(term_id)
    }

    pub fn set_term_tags(&self, term_id: i64, tags: &[String]) -> Result<Term> {
        let n = self.conn.execute(
            "UPDATE terms SET tags = ?1, updated_at = ?2 WHERE id = ?3",
            params![serde_json::to_string(&clean_list(tags))?, now_iso(), term_id],
        )?;
        if n == 0 {
            return Err(Error::NotFound(format!("用語 ID {term_id}")));
        }
        self.get_term(term_id)
    }

    pub fn all_tags(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT j.value FROM terms, json_each(terms.tags) j ORDER BY 1")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// 用語ごとの判定数（各問題の最新の判定で数える）。
    fn term_counts(&self) -> Result<HashMap<i64, VerdictCounts>> {
        let mut stmt = self.conn.prepare(
            "SELECT q.term_id,
                (SELECT a.verdict FROM attempts a WHERE a.question_id = q.id ORDER BY a.id DESC LIMIT 1) AS v
             FROM questions q",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))?;
        let mut map: HashMap<i64, VerdictCounts> = HashMap::new();
        for row in rows {
            let (tid, v) = row?;
            if let Some(v) = v {
                map.entry(tid).or_default().add(Verdict::parse(&v)?);
            }
        }
        Ok(map)
    }

    pub fn list_terms(&self, tag: Option<&str>, query: Option<&str>) -> Result<Vec<TermListItem>> {
        let counts = self.term_counts()?;
        let mut stmt = self.conn.prepare(
            "SELECT t.*,
                (SELECT max(q.created_at) FROM questions q WHERE q.term_id = t.id) AS last_asked_at,
                (SELECT min(r.due_date) FROM review_states r WHERE r.term_id = t.id) AS next_due
             FROM terms t ORDER BY t.headword COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((term_from_row(r)?, r.get::<_, Option<String>>("last_asked_at")?, r.get::<_, Option<String>>("next_due")?))
        })?;
        let qkey = query.map(term_key).filter(|k| !k.is_empty());
        let mut out = Vec::new();
        for row in rows {
            let (term, last_asked_at, next_due) = row?;
            if tag.is_some_and(|tg| !term.tags.iter().any(|t| t == tg)) {
                continue;
            }
            if let Some(k) = &qkey {
                let hit = std::iter::once(&term.headword).chain(term.aliases.iter()).any(|n| term_key(n).contains(k.as_str()));
                if !hit {
                    continue;
                }
            }
            let c = counts.get(&term.id).cloned().unwrap_or_default();
            out.push(TermListItem { accuracy: c.accuracy(), counts: c, term, last_asked_at, next_due });
        }
        Ok(out)
    }

    pub fn delete_term(&self, term_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM terms WHERE id = ?1", [term_id])?;
        Ok(())
    }

    // ------------------------------------------------------------------ セッション

    pub fn start_session(&self, format: SessionFormat, theme: &str) -> Result<Session> {
        self.conn.execute(
            "INSERT INTO sessions(format, theme, started_at) VALUES (?1, ?2, ?3)",
            params![format.as_str(), theme.trim(), now_iso()],
        )?;
        self.get_session(self.conn.last_insert_rowid())
    }

    pub fn get_session(&self, id: i64) -> Result<Session> {
        self.conn
            .query_row("SELECT * FROM sessions WHERE id = ?1", [id], session_from_row)
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("セッション ID {id}")))
    }

    fn session_counts(&self, session_id: i64) -> Result<(VerdictCounts, i64)> {
        let qs = self.session_questions(session_id)?;
        let mut c = VerdictCounts::default();
        for q in &qs {
            if let Some(v) = q.latest_verdict {
                c.add(v);
            }
        }
        Ok((c, qs.len() as i64))
    }

    /// セッション終了。まとめ文を保存し、正答率（部分正解は 0.5 点）を返す。
    pub fn end_session(&self, session_id: i64, summary: &str) -> Result<SessionEnd> {
        self.get_session(session_id)?;
        let (counts, question_count) = self.session_counts(session_id)?;
        let accuracy = counts.accuracy();
        self.conn.execute(
            "UPDATE sessions SET ended_at = ?1, summary = ?2, accuracy = ?3 WHERE id = ?4",
            params![now_iso(), summary.trim(), accuracy, session_id],
        )?;
        Ok(SessionEnd { session_id, accuracy, counts, question_count })
    }

    pub fn session_questions(&self, session_id: i64) -> Result<Vec<QuestionSummary>> {
        let sql = format!("{SUMMARY_SELECT} WHERE q.session_id = ?1 ORDER BY q.id");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([session_id], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn session_detail(&self, session_id: i64) -> Result<SessionDetail> {
        Ok(SessionDetail { session: self.get_session(session_id)?, questions: self.session_questions(session_id)? })
    }

    /// 履歴一覧（セッション単位・新しい順）。形式・タグ・正誤で問題を絞り込み、該当がないセッションは省く。
    pub fn history(&self, filter: &HistoryFilter) -> Result<Vec<HistoryGroup>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 500);
        let offset = filter.offset.unwrap_or(0).max(0);
        let filtering = filter.format.is_some() || filter.tag.is_some() || filter.verdict.is_some();

        let mut tags_by_term: HashMap<i64, Vec<String>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare("SELECT id, tags FROM terms")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, json_vec(r.get(1)?))))?;
            for row in rows {
                let (id, tags) = row?;
                tags_by_term.insert(id, tags);
            }
        }

        // 問題をまとめて読み、セッションごとに分ける
        let mut by_session: HashMap<i64, Vec<QuestionSummary>> = HashMap::new();
        {
            let sql = format!("{SUMMARY_SELECT} ORDER BY q.id");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map([], summary_from_row)?;
            for row in rows {
                let q = row?;
                by_session.entry(q.question.session_id).or_default().push(q);
            }
        }

        let mut stmt = self.conn.prepare("SELECT * FROM sessions ORDER BY started_at DESC, id DESC")?;
        let sessions = stmt.query_map([], session_from_row)?;
        let mut out = Vec::new();
        let mut skipped = 0;
        for s in sessions {
            let session = s?;
            let all = by_session.remove(&session.id).unwrap_or_default();
            let questions: Vec<QuestionSummary> = all
                .iter()
                .filter(|q| filter.format.is_none_or(|f| q.question.format == f))
                .filter(|q| filter.verdict.is_none_or(|v| q.latest_verdict == Some(v)))
                .filter(|q| {
                    filter.tag.as_ref().is_none_or(|tg| {
                        tags_by_term.get(&q.question.term_id).is_some_and(|ts| ts.iter().any(|t| t == tg))
                    })
                })
                .cloned()
                .collect();
            if filtering && questions.is_empty() {
                continue;
            }
            if skipped < offset {
                skipped += 1;
                continue;
            }
            let mut c = VerdictCounts::default();
            let mut tags = BTreeSet::new();
            for q in &all {
                if let Some(v) = q.latest_verdict {
                    c.add(v);
                }
                if let Some(ts) = tags_by_term.get(&q.question.term_id) {
                    tags.extend(ts.iter().cloned());
                }
            }
            out.push(HistoryGroup {
                session: SessionListItem {
                    question_count: all.len() as i64,
                    correct: c.correct,
                    partial: c.partial,
                    incorrect: c.incorrect,
                    tags: tags.into_iter().collect(),
                    session,
                },
                questions,
            });
            if out.len() as i64 >= limit {
                break;
            }
        }
        Ok(out)
    }

    pub fn delete_session(&self, session_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", [session_id])?;
        Ok(())
    }

    // ------------------------------------------------------------------ 問題・回答

    pub fn get_question(&self, id: i64) -> Result<Question> {
        self.conn
            .query_row("SELECT * FROM questions WHERE id = ?1", [id], question_from_row)
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("問題 ID {id}")))
    }

    pub fn record_question(&self, input: &RecordQuestion) -> Result<Question> {
        // 用語の登録と問題の保存をまとめて行い、途中で失敗したら用語の登録も取り消す
        self.in_savepoint(|| self.insert_question(input))
    }

    fn insert_question(&self, input: &RecordQuestion) -> Result<Question> {
        let session = self.get_session(input.session_id)?;
        if input.prompt.trim().is_empty() {
            return Err(Error::Invalid("問題文（prompt）が空です".into()));
        }
        let parent = input.parent_id.map(|p| self.get_question(p)).transpose()?;
        if let Some(p) = &parent {
            if p.session_id != session.id {
                return Err(Error::Invalid(format!(
                    "親問題 {} は別のセッション（{}）の問題です",
                    p.id, p.session_id
                )));
            }
        }
        let format = match (&parent, input.format, session.format.single()) {
            (Some(p), _, _) => p.format,
            (None, Some(f), _) => f,
            (None, None, Some(f)) => f,
            (None, None, None) => {
                return Err(Error::Invalid("混合セッションでは format を指定してください".into()))
            }
        };
        let term = match (&input.term, &parent) {
            (Some(t), _) => self.resolve_term(t)?,
            (None, Some(p)) => self.get_term(p.term_id)?,
            (None, None) => return Err(Error::Invalid("用語（term）を指定してください".into())),
        };
        let depth = parent.as_ref().map(|p| p.depth + 1).unwrap_or(0);
        self.conn.execute(
            "INSERT INTO questions(session_id, term_id, format, prompt, model_answer, parent_id, depth, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.id,
                term.id,
                format.as_str(),
                input.prompt.trim(),
                input.model_answer.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                input.parent_id,
                depth,
                now_iso()
            ],
        )?;
        self.get_question(self.conn.last_insert_rowid())
    }

    /// 回答・判定・解説を保存し、復習スケジュールを更新する。図解データは保存前に検査する。
    pub fn record_attempt(&self, input: &RecordAttempt, today: NaiveDate) -> Result<AttemptResult> {
        let q = self.get_question(input.question_id)?;
        let report = diagram::validate_all(&input.diagrams);
        if !report.errors.is_empty() {
            return Err(Error::Diagram(report.errors));
        }
        for s in &input.sources {
            if let Some(u) = &s.url {
                let lower = u.to_ascii_lowercase();
                if !(lower.starts_with("https://") || lower.starts_with("http://")) {
                    return Err(Error::Invalid(format!("出典の URL は http(s) のみです: {u}")));
                }
            }
        }

        self.in_savepoint(|| self.insert_attempt(&q, input, today, report.warnings))
    }

    /// 入れ子にできるトランザクション（呼び出し側がすでに BEGIN していても使える）。
    fn in_savepoint<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.conn.execute_batch("SAVEPOINT mondeep_sp")?;
        match f() {
            Ok(v) => {
                self.conn.execute_batch("RELEASE mondeep_sp")?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK TO mondeep_sp; RELEASE mondeep_sp");
                Err(e)
            }
        }
    }

    fn insert_attempt(
        &self,
        q: &Question,
        input: &RecordAttempt,
        today: NaiveDate,
        warnings: Vec<String>,
    ) -> Result<AttemptResult> {
        let tx = &self.conn;
        let now = now_iso();
        tx.execute(
            "INSERT INTO attempts(question_id, answer, verdict, reason, answered_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![q.id, input.answer, input.verdict.as_str(), input.reason.trim(), now],
        )?;
        let attempt_id = tx.last_insert_rowid();
        let md = input.explanation_md.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let explanation_id = if md.is_some() || !input.diagrams.is_empty() {
            tx.execute(
                "INSERT INTO explanations(question_id, attempt_id, body_md, diagrams, sources, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    q.id,
                    attempt_id,
                    md.unwrap_or(""),
                    serde_json::to_string(&input.diagrams)?,
                    serde_json::to_string(&input.sources)?,
                    now
                ],
            )?;
            Some(tx.last_insert_rowid())
        } else {
            None
        };
        let review = self.update_review(q, today)?;
        Ok(AttemptResult { attempt_id, explanation_id, review, warnings })
    }

    fn root_of(&self, q: &Question) -> Result<Question> {
        let mut cur = q.clone();
        let mut guard = 0;
        while let Some(p) = cur.parent_id {
            cur = self.get_question(p)?;
            guard += 1;
            if guard > 100 {
                return Err(Error::Invalid("深掘りの木が深すぎます".into()));
            }
        }
        Ok(cur)
    }

    /// 根から下の全問題（自分を含む）。
    fn tree_of(&self, root_id: i64) -> Result<Vec<QuestionSummary>> {
        let sql = format!(
            "WITH RECURSIVE tree(id) AS (
                SELECT ?1 UNION ALL SELECT q.id FROM questions q JOIN tree ON q.parent_id = tree.id
            )
            {SUMMARY_SELECT} WHERE q.id IN (SELECT id FROM tree) ORDER BY q.depth, q.id"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([root_id], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// 深掘りの木の評価（最初の問いの判定と、正解できた最も深い段）から復習状態を更新する。
    ///
    /// 同じ木の中で回答が増えるたびに呼ばれるので、木の最初の回答の直前の状態（tree_prev）を控えておき、
    /// そこから計算し直す。これで 1 本の木は 1 回の復習として数えられる。
    fn update_review(&self, q: &Question, today: NaiveDate) -> Result<ReviewState> {
        let root = self.root_of(q)?;
        let tree = self.tree_of(root.id)?;
        let root_verdict = tree
            .iter()
            .find(|t| t.question.id == root.id)
            .and_then(|t| t.latest_verdict)
            .or_else(|| tree.iter().find(|t| t.question.id == q.id).and_then(|t| t.latest_verdict))
            .unwrap_or(Verdict::Incorrect);
        let max_correct_depth = tree
            .iter()
            .filter(|t| t.latest_verdict == Some(Verdict::Correct))
            .map(|t| t.question.depth)
            .max();
        let quality = srs::quality(root.format, root_verdict, max_correct_depth);

        let existing: Option<StoredReview> = self
            .conn
            .query_row(
                "SELECT due_date, interval_days, ease, streak, tree_root_id, tree_prev
                 FROM review_states WHERE term_id = ?1 AND format = ?2",
                params![root.term_id, root.format.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()?;
        let base = match existing {
            None => Schedule::new(today),
            Some((_, _, _, _, Some(tree_root), Some(prev))) if tree_root == root.id => serde_json::from_str(&prev)?,
            Some((due, interval_days, ease, streak, _, _)) => {
                Schedule { interval_days, ease, streak, due_date: parse_date(&due)? }
            }
        };
        let next = srs::next(base, quality, today);
        self.conn.execute(
            "INSERT INTO review_states(term_id, format, due_date, interval_days, ease, streak, last_reviewed_at, tree_root_id, tree_prev)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(term_id, format) DO UPDATE SET
                due_date = excluded.due_date, interval_days = excluded.interval_days, ease = excluded.ease,
                streak = excluded.streak, last_reviewed_at = excluded.last_reviewed_at,
                tree_root_id = excluded.tree_root_id, tree_prev = excluded.tree_prev",
            params![
                root.term_id,
                root.format.as_str(),
                date_str(next.due_date),
                next.interval_days,
                next.ease,
                next.streak,
                now_iso(),
                root.id,
                serde_json::to_string(&base)?
            ],
        )?;
        // 復習で答えられたら要復習の印は外す
        if quality >= 4 {
            self.conn.execute("UPDATE terms SET needs_review = 0 WHERE id = ?1", [root.term_id])?;
        }
        self.review_state(root.term_id, root.format)?
            .ok_or_else(|| Error::NotFound("復習状態".into()))
    }

    pub fn review_state(&self, term_id: i64, format: Format) -> Result<Option<ReviewState>> {
        Ok(self
            .conn
            .query_row(
                "SELECT * FROM review_states WHERE term_id = ?1 AND format = ?2",
                params![term_id, format.as_str()],
                review_from_row,
            )
            .optional()?)
    }

    fn review_states_of(&self, term_id: i64) -> Result<Vec<ReviewState>> {
        let mut stmt = self.conn.prepare("SELECT * FROM review_states WHERE term_id = ?1 ORDER BY format")?;
        let rows = stmt.query_map([term_id], review_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn question_detail(&self, id: i64) -> Result<QuestionDetail> {
        let question = self.get_question(id)?;
        let term = self.get_term(question.term_id)?;
        let session = self.get_session(question.session_id)?;
        let mut stmt = self.conn.prepare("SELECT * FROM attempts WHERE question_id = ?1 ORDER BY id")?;
        let attempts = stmt.query_map([id], attempt_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        let explanation = self
            .conn
            .query_row(
                "SELECT * FROM explanations WHERE question_id = ?1 ORDER BY id DESC LIMIT 1",
                [id],
                explanation_from_row,
            )
            .optional()?;
        let root = self.root_of(&question)?;
        let tree = self.tree_of(root.id)?;
        Ok(QuestionDetail { question, term, session, attempts, explanation, tree, root_id: root.id })
    }

    // ------------------------------------------------------------------ 用語カード・履歴

    fn term_questions(&self, term_id: i64) -> Result<Vec<QuestionSummary>> {
        let sql = format!("{SUMMARY_SELECT} WHERE q.term_id = ?1 ORDER BY q.id DESC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([term_id], summary_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn term_card(&self, term_id: i64) -> Result<TermCard> {
        let term = self.get_term(term_id)?;
        let questions = self.term_questions(term_id)?;
        let mut counts = VerdictCounts::default();
        for q in &questions {
            if let Some(v) = q.latest_verdict {
                counts.add(v);
            }
        }
        let latest_explanation = self
            .conn
            .query_row(
                "SELECT e.* FROM explanations e JOIN questions q ON q.id = e.question_id
                 WHERE q.term_id = ?1 ORDER BY e.id DESC LIMIT 1",
                [term_id],
                explanation_from_row,
            )
            .optional()?;
        let max_correct_depth = questions
            .iter()
            .filter(|q| q.question.format == Format::TermToMeaning && q.latest_verdict == Some(Verdict::Correct))
            .map(|q| q.question.depth)
            .max();
        Ok(TermCard {
            accuracy: counts.accuracy(),
            counts,
            review_states: self.review_states_of(term_id)?,
            questions,
            latest_explanation,
            max_correct_depth,
            term,
        })
    }

    /// 用語の過去の出題・回答（MCP の get_term_history）。
    pub fn term_history(&self, term_id: i64, limit: i64) -> Result<TermHistory> {
        let term = self.get_term(term_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT q.*, s.theme,
                a.answer AS latest_answer, a.verdict AS latest_verdict, a.reason AS latest_reason
             FROM questions q
             JOIN sessions s ON s.id = q.session_id
             LEFT JOIN attempts a ON a.id = (SELECT max(id) FROM attempts WHERE question_id = q.id)
             WHERE q.term_id = ?1 ORDER BY q.id DESC LIMIT ?2",
        )?;
        let entries = stmt
            .query_map(params![term_id, limit.clamp(1, 200)], |r| {
                let q = question_from_row(r)?;
                Ok(HistoryEntry {
                    question_id: q.id,
                    session_id: q.session_id,
                    session_theme: r.get("theme")?,
                    format: q.format,
                    depth: q.depth,
                    parent_id: q.parent_id,
                    prompt: q.prompt,
                    asked_at: q.created_at,
                    latest_answer: r.get("latest_answer")?,
                    latest_verdict: opt_verdict(r.get("latest_verdict")?)?,
                    latest_reason: r.get("latest_reason")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let card = self.term_card(term_id)?;
        Ok(TermHistory { term, counts: card.counts, review_states: card.review_states, entries })
    }

    // ------------------------------------------------------------------ 復習・苦手

    /// 今日復習すべき用語。次回復習日が今日以前のもの、または要復習の印があるもの。
    pub fn review_queue(&self, limit: i64, tags: &[String], today: NaiveDate) -> Result<Vec<ReviewItem>> {
        let counts = self.term_counts()?;
        let today_s = date_str(today);
        let mut items: Vec<ReviewItem> = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT r.*, t.headword, t.aliases, t.tags, t.needs_review FROM review_states r
             JOIN terms t ON t.id = r.term_id
             WHERE r.due_date <= ?1 OR t.needs_review = 1
             ORDER BY t.needs_review DESC, r.due_date ASC, r.term_id",
        )?;
        let rows = stmt.query_map([&today_s], |r| {
            Ok((
                review_from_row(r)?,
                r.get::<_, String>("headword")?,
                json_vec(r.get("aliases")?),
                json_vec(r.get("tags")?),
                r.get::<_, i64>("needs_review")? != 0,
            ))
        })?;
        for row in rows {
            let (rs, headword, aliases, ttags, needs_review) = row?;
            // 印だけで期日前の形式は、同じ用語の別の形式がすでに入っていれば重ねない
            if rs.due_date > today_s && items.iter().any(|i| i.term_id == rs.term_id) {
                continue;
            }
            items.push(ReviewItem {
                term_id: rs.term_id,
                headword,
                aliases,
                tags: ttags,
                format: Some(rs.format),
                due_date: Some(rs.due_date),
                needs_review,
                counts: counts.get(&rs.term_id).cloned().unwrap_or_default(),
                last_verdict: None,
                summary: String::new(),
            });
        }
        // 要復習の印はあるが、まだ復習状態がない用語
        let mut stmt = self.conn.prepare(
            "SELECT * FROM terms t WHERE t.needs_review = 1
             AND NOT EXISTS (SELECT 1 FROM review_states r WHERE r.term_id = t.id) ORDER BY t.id",
        )?;
        let rows = stmt.query_map([], term_from_row)?;
        for row in rows {
            let t = row?;
            items.insert(
                0,
                ReviewItem {
                    term_id: t.id,
                    headword: t.headword,
                    aliases: t.aliases,
                    tags: t.tags,
                    format: None,
                    due_date: None,
                    needs_review: true,
                    counts: counts.get(&t.id).cloned().unwrap_or_default(),
                    last_verdict: None,
                    summary: String::new(),
                },
            );
        }
        let tags = clean_list(tags);
        items.retain(|i| tags.is_empty() || i.tags.iter().any(|t| tags.contains(t)));
        items.truncate(limit.clamp(1, 200) as usize);
        for item in &mut items {
            let last: Option<(String, String)> = self
                .conn
                .query_row(
                    "SELECT a.verdict, a.answered_at FROM attempts a JOIN questions q ON q.id = a.question_id
                     WHERE q.term_id = ?1 AND (?2 IS NULL OR q.format = ?2) ORDER BY a.id DESC LIMIT 1",
                    params![item.term_id, item.format.map(|f| f.as_str())],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            item.last_verdict = last.as_ref().map(|(v, _)| Verdict::parse(v)).transpose()?;
            let c = &item.counts;
            let mut s = format!(
                "過去 {} 問: 正解 {} / 部分正解 {} / 不正解 {}",
                c.total(),
                c.correct,
                c.partial,
                c.incorrect
            );
            if let (Some(v), Some((_, at))) = (item.last_verdict, &last) {
                let label = match v {
                    Verdict::Correct => "正解",
                    Verdict::Partial => "部分正解",
                    Verdict::Incorrect => "不正解",
                };
                s.push_str(&format!("。前回は{label}（{}）", &at[..at.len().min(10)]));
            }
            if item.needs_review {
                s.push_str("。要復習の印あり");
            }
            item.summary = s;
        }
        Ok(items)
    }

    /// 苦手用語。不正解が多い用語と、深掘りが浅い段で止まる用語、要復習の印がある用語。
    pub fn weak_terms(&self, limit: i64, tags: &[String]) -> Result<Vec<WeakTerm>> {
        let counts = self.term_counts()?;
        let tags = clean_list(tags);
        // 深掘りの木ごとに「正解できた最も深い段」（なければ -1）を求める
        let mut tree_depths: HashMap<i64, Vec<i64>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT q.id, q.term_id FROM questions q
                 WHERE q.parent_id IS NULL AND q.format = 'term_to_meaning'
                 AND EXISTS (SELECT 1 FROM questions c WHERE c.parent_id = q.id)",
            )?;
            let roots = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (root_id, term_id) in roots {
                let tree = self.tree_of(root_id)?;
                let deepest = tree
                    .iter()
                    .filter(|t| t.latest_verdict == Some(Verdict::Correct))
                    .map(|t| t.question.depth)
                    .max()
                    .unwrap_or(-1);
                tree_depths.entry(term_id).or_default().push(deepest);
            }
        }
        let mut stmt = self.conn.prepare("SELECT * FROM terms")?;
        let terms = stmt.query_map([], term_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut out = Vec::new();
        for t in terms {
            if !tags.is_empty() && !t.tags.iter().any(|x| tags.contains(x)) {
                continue;
            }
            let c = counts.get(&t.id).cloned().unwrap_or_default();
            let accuracy = c.accuracy();
            let avg_depth = tree_depths
                .get(&t.id)
                .map(|ds| ds.iter().sum::<i64>() as f64 / ds.len() as f64);
            let mut reasons = Vec::new();
            let mut score = 0.0;
            if c.incorrect + c.partial > 0 && accuracy.is_some_and(|a| a < 0.8) {
                reasons.push(format!(
                    "{} 問中 不正解 {}・部分正解 {}（正答率 {:.0}%）",
                    c.total(),
                    c.incorrect,
                    c.partial,
                    accuracy.unwrap_or(0.0) * 100.0
                ));
                score += c.incorrect as f64 + 0.5 * c.partial as f64 + (1.0 - accuracy.unwrap_or(0.0)) * 2.0;
            }
            if let Some(d) = avg_depth {
                if d < 1.5 {
                    reasons.push(if d < 0.0 {
                        "深掘りで最初の問いから詰まる".to_string()
                    } else {
                        format!("深掘りが浅い段で止まりがち（正解できた最も深い段の平均 {d:.1}、最初の問いが 0）")
                    });
                    score += 1.5 - d.max(-1.0);
                }
            }
            if t.needs_review {
                reasons.push("要復習の印あり".to_string());
                score += 2.0;
            }
            if reasons.is_empty() {
                continue;
            }
            out.push(WeakTerm {
                term_id: t.id,
                headword: t.headword,
                tags: t.tags,
                needs_review: t.needs_review,
                counts: c,
                accuracy,
                avg_max_correct_depth: avg_depth,
                score,
                reasons,
            });
        }
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(limit.clamp(1, 200) as usize);
        Ok(out)
    }

    // ------------------------------------------------------------------ 図の微調整

    /// 用語の図の微調整をすべて返す（アプリが描画時に重ねる）。
    pub fn diagram_adjustments(&self, term_id: i64) -> Result<Vec<DiagramAdjustment>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM diagram_adjustments WHERE term_id = ?1 ORDER BY kind, element_id",
        )?;
        let rows = stmt.query_map([term_id], |r| {
            Ok(DiagramAdjustment {
                term_id: r.get("term_id")?,
                kind: r.get("kind")?,
                element_id: r.get("element_id")?,
                dx: r.get("dx")?,
                dy: r.get("dy")?,
                updated_at: r.get("updated_at")?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// 部品 1 つの微調整を保存する。dx・dy がどちらも 0 なら消す。
    pub fn set_diagram_adjustment(&self, term_id: i64, kind: &str, element_id: &str, dx: f64, dy: f64) -> Result<()> {
        if !matches!(kind, "concept" | "state") {
            return Err(Error::Invalid(format!("位置を調整できるのは概念図と状態遷移図だけです（{kind}）")));
        }
        if element_id.trim().is_empty() || element_id.len() > 200 {
            return Err(Error::Invalid("部品 ID が不正です".into()));
        }
        if !dx.is_finite() || !dy.is_finite() {
            return Err(Error::Invalid("移動量が不正です".into()));
        }
        self.get_term(term_id)?;
        let (dx, dy) = (dx.clamp(-MAX_ADJUST, MAX_ADJUST).round(), dy.clamp(-MAX_ADJUST, MAX_ADJUST).round());
        if dx == 0.0 && dy == 0.0 {
            self.conn.execute(
                "DELETE FROM diagram_adjustments WHERE term_id = ?1 AND kind = ?2 AND element_id = ?3",
                params![term_id, kind, element_id],
            )?;
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO diagram_adjustments(term_id, kind, element_id, dx, dy, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(term_id, kind, element_id) DO UPDATE SET dx = excluded.dx, dy = excluded.dy, updated_at = excluded.updated_at",
            params![term_id, kind, element_id, dx, dy, now_iso()],
        )?;
        Ok(())
    }

    /// 図の種類ごとに微調整を消して、自動配置に戻す。
    pub fn reset_diagram_adjustments(&self, term_id: i64, kind: &str) -> Result<usize> {
        Ok(self.conn.execute(
            "DELETE FROM diagram_adjustments WHERE term_id = ?1 AND kind = ?2",
            params![term_id, kind],
        )?)
    }

    // ------------------------------------------------------------------ 検索

    /// 用語・問題文・解説の全文検索（trigram 索引で LIKE 部分一致）。
    pub fn search(&self, keyword: &str, limit: i64) -> Result<Vec<SearchHit>> {
        let kw = keyword.trim();
        if kw.is_empty() {
            return Ok(Vec::new());
        }
        let escaped = kw.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let mut stmt = self.conn.prepare(
            "SELECT s.kind, s.ref_id, s.term_id, s.body, t.headword,
                CASE s.kind WHEN 'question' THEN s.ref_id
                            WHEN 'explanation' THEN (SELECT question_id FROM explanations WHERE id = s.ref_id)
                            ELSE NULL END AS question_id
             FROM search_index s JOIN terms t ON t.id = s.term_id
             WHERE s.body LIKE ?1 ESCAPE '\\'
             ORDER BY CASE s.kind WHEN 'term' THEN 0 WHEN 'question' THEN 1 ELSE 2 END, s.rowid DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit.clamp(1, 500)], |r| {
            let kind: String = r.get(0)?;
            let body: String = r.get(3)?;
            let body = if kind == "term" { body.replace(['[', ']', '"'], "").replace(',', "、") } else { body };
            Ok(SearchHit {
                kind,
                term_id: r.get(2)?,
                headword: r.get(4)?,
                question_id: r.get(5)?,
                snippet: snippet(&body, kw, 40),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// キーワードの前後 `radius` 文字を切り出す。
pub fn snippet(body: &str, keyword: &str, radius: usize) -> String {
    let lower_body: Vec<char> = body.chars().flat_map(|c| c.to_lowercase()).collect();
    let lower_kw: Vec<char> = keyword.chars().flat_map(|c| c.to_lowercase()).collect();
    let chars: Vec<char> = body.chars().collect();
    let pos = if lower_body.len() == chars.len() && !lower_kw.is_empty() {
        lower_body.windows(lower_kw.len()).position(|w| w == lower_kw.as_slice())
    } else {
        None
    };
    let (start, end) = match pos {
        Some(p) => (p.saturating_sub(radius), (p + lower_kw.len() + radius).min(chars.len())),
        None => (0, (radius * 2).min(chars.len())),
    };
    let mut s: String = chars[start..end].iter().collect::<String>().replace('\n', " ");
    if start > 0 {
        s.insert(0, '…');
    }
    if end < chars.len() {
        s.push('…');
    }
    s
}
