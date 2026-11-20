//! SQLite の接続とマイグレーション。
//!
//! アプリと MCP サーバーが同じファイルを同時に開くので WAL モードにし、busy_timeout を長めに取る。
//! スキーマの版は `PRAGMA user_version` で管理し、`MIGRATIONS` を順に流す。
use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::Result;

/// マイグレーション。追加するときは末尾に足すだけにし、既存のものは書き換えない。
pub const MIGRATIONS: &[&str] = &[
    // v1: 初期スキーマ
    r#"
    CREATE TABLE terms (
        id            INTEGER PRIMARY KEY,
        headword      TEXT NOT NULL,
        aliases       TEXT NOT NULL DEFAULT '[]',
        tags          TEXT NOT NULL DEFAULT '[]',
        needs_review  INTEGER NOT NULL DEFAULT 0,
        created_at    TEXT NOT NULL,
        updated_at    TEXT NOT NULL
    );
    -- 見出し語と別表記の照合キー。同じキーは 1 つの用語にしか属さない
    CREATE TABLE term_keys (
        key      TEXT PRIMARY KEY,
        term_id  INTEGER NOT NULL REFERENCES terms(id) ON DELETE CASCADE
    );
    CREATE INDEX idx_term_keys_term ON term_keys(term_id);

    CREATE TABLE sessions (
        id          INTEGER PRIMARY KEY,
        format      TEXT NOT NULL CHECK (format IN ('meaning_to_term', 'term_to_meaning', 'mixed')),
        theme       TEXT NOT NULL DEFAULT '',
        started_at  TEXT NOT NULL,
        ended_at    TEXT,
        summary     TEXT,
        accuracy    REAL
    );
    CREATE INDEX idx_sessions_started ON sessions(started_at);

    CREATE TABLE questions (
        id            INTEGER PRIMARY KEY,
        session_id    INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        term_id       INTEGER NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
        format        TEXT NOT NULL CHECK (format IN ('meaning_to_term', 'term_to_meaning')),
        prompt        TEXT NOT NULL,
        model_answer  TEXT,
        parent_id     INTEGER REFERENCES questions(id) ON DELETE CASCADE,
        depth         INTEGER NOT NULL DEFAULT 0,
        created_at    TEXT NOT NULL
    );
    CREATE INDEX idx_questions_session ON questions(session_id);
    CREATE INDEX idx_questions_term ON questions(term_id);
    CREATE INDEX idx_questions_parent ON questions(parent_id);

    CREATE TABLE attempts (
        id           INTEGER PRIMARY KEY,
        question_id  INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
        answer       TEXT NOT NULL,
        verdict      TEXT NOT NULL CHECK (verdict IN ('correct', 'partial', 'incorrect')),
        reason       TEXT NOT NULL DEFAULT '',
        answered_at  TEXT NOT NULL
    );
    CREATE INDEX idx_attempts_question ON attempts(question_id);

    CREATE TABLE explanations (
        id           INTEGER PRIMARY KEY,
        question_id  INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
        attempt_id   INTEGER REFERENCES attempts(id) ON DELETE SET NULL,
        body_md      TEXT NOT NULL,
        diagrams     TEXT NOT NULL DEFAULT '[]',
        sources      TEXT NOT NULL DEFAULT '[]',
        created_at   TEXT NOT NULL
    );
    CREATE INDEX idx_explanations_question ON explanations(question_id);

    -- 復習状態は「用語 × 形式」ごと。tree_* は深掘りの木 1 本ぶんの評価をやり直すための控え
    CREATE TABLE review_states (
        term_id           INTEGER NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
        format            TEXT NOT NULL CHECK (format IN ('meaning_to_term', 'term_to_meaning')),
        due_date          TEXT NOT NULL,
        interval_days     INTEGER NOT NULL,
        ease              REAL NOT NULL,
        streak            INTEGER NOT NULL,
        last_reviewed_at  TEXT,
        tree_root_id      INTEGER,
        tree_prev         TEXT,
        PRIMARY KEY (term_id, format)
    );
    CREATE INDEX idx_review_due ON review_states(due_date);

    -- 全文検索（日本語でも引けるよう trigram）。kind: term / question / explanation
    CREATE VIRTUAL TABLE search_index USING fts5(
        kind UNINDEXED, ref_id UNINDEXED, term_id UNINDEXED, body,
        tokenize = 'trigram'
    );

    CREATE TRIGGER trg_terms_ai AFTER INSERT ON terms BEGIN
        INSERT INTO search_index(kind, ref_id, term_id, body)
        VALUES ('term', new.id, new.id, new.headword || ' ' || new.aliases);
    END;
    CREATE TRIGGER trg_terms_au AFTER UPDATE OF headword, aliases ON terms BEGIN
        DELETE FROM search_index WHERE kind = 'term' AND ref_id = old.id;
        INSERT INTO search_index(kind, ref_id, term_id, body)
        VALUES ('term', new.id, new.id, new.headword || ' ' || new.aliases);
    END;
    CREATE TRIGGER trg_terms_ad AFTER DELETE ON terms BEGIN
        DELETE FROM search_index WHERE kind = 'term' AND ref_id = old.id;
    END;
    CREATE TRIGGER trg_questions_ai AFTER INSERT ON questions BEGIN
        INSERT INTO search_index(kind, ref_id, term_id, body)
        VALUES ('question', new.id, new.term_id, new.prompt || ' ' || coalesce(new.model_answer, ''));
    END;
    CREATE TRIGGER trg_questions_ad AFTER DELETE ON questions BEGIN
        DELETE FROM search_index WHERE kind = 'question' AND ref_id = old.id;
    END;
    CREATE TRIGGER trg_explanations_ai AFTER INSERT ON explanations BEGIN
        INSERT INTO search_index(kind, ref_id, term_id, body)
        SELECT 'explanation', new.id, q.term_id, new.body_md FROM questions q WHERE q.id = new.question_id;
    END;
    CREATE TRIGGER trg_explanations_ad AFTER DELETE ON explanations BEGIN
        DELETE FROM search_index WHERE kind = 'explanation' AND ref_id = old.id;
    END;
    "#,
    // v2: 図の位置の微調整（部品ごと）。用語 × 図の種類 × 部品 ID で持つので、
    // Claude が同じ用語の図を描き直しても、同じ ID の部品には微調整が引き継がれる
    r#"
    CREATE TABLE diagram_adjustments (
        term_id     INTEGER NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
        kind        TEXT NOT NULL CHECK (kind IN ('concept', 'state')),
        element_id  TEXT NOT NULL,
        dx          REAL NOT NULL,
        dy          REAL NOT NULL,
        updated_at  TEXT NOT NULL,
        PRIMARY KEY (term_id, kind, element_id)
    );
    "#,
    // v3: 手動修正（判定の上書きと、問題ごとのメモ）。上書き前の判定は original_verdict に残す
    r#"
    ALTER TABLE attempts ADD COLUMN original_verdict TEXT
        CHECK (original_verdict IS NULL OR original_verdict IN ('correct', 'partial', 'incorrect'));
    ALTER TABLE attempts ADD COLUMN overridden_at TEXT;
    ALTER TABLE questions ADD COLUMN note TEXT;
    ALTER TABLE questions ADD COLUMN note_updated_at TEXT;

    CREATE TRIGGER trg_questions_note AFTER UPDATE OF note ON questions BEGIN
        DELETE FROM search_index WHERE kind = 'note' AND ref_id = new.id;
        INSERT INTO search_index(kind, ref_id, term_id, body)
        SELECT 'note', new.id, new.term_id, new.note WHERE coalesce(new.note, '') <> '';
    END;
    CREATE TRIGGER trg_questions_ad_note AFTER DELETE ON questions BEGIN
        DELETE FROM search_index WHERE kind = 'note' AND ref_id = old.id;
    END;
    "#,
];

/// DB を開き、WAL・外部キーを有効にしてマイグレーションを流す。親フォルダがなければ作る。
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

/// テスト用のメモリ DB。
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    let current = schema_version(conn)? as usize;
    if current > MIGRATIONS.len() {
        return Err(crate::Error::Invalid(format!(
            "DB の版（{current}）がこのアプリ（{}）より新しいです。アプリを更新してください",
            MIGRATIONS.len()
        )));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        // 同時に起動したアプリと MCP サーバーが二重に流さないよう、排他トランザクションで版を確かめ直す
        conn.execute_batch("BEGIN EXCLUSIVE")?;
        let v = schema_version(conn)? as usize;
        if v > i {
            conn.execute_batch("COMMIT")?;
            continue;
        }
        let res = conn
            .execute_batch(sql)
            .and_then(|_| conn.pragma_update(None, "user_version", (i + 1) as i64));
        match res {
            Ok(()) => conn.execute_batch("COMMIT")?,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e.into());
            }
        }
    }
    Ok(())
}

/// 他の接続（MCP サーバー）が書き込むと値が変わる。アプリの即時反映に使う。
pub fn data_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA data_version", [], |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub").join("m.db");
        let c = open(&p).unwrap();
        assert_eq!(schema_version(&c).unwrap(), MIGRATIONS.len() as i64);
        let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        drop(c);
        let c = open(&p).unwrap();
        assert_eq!(schema_version(&c).unwrap(), MIGRATIONS.len() as i64);
    }

    #[test]
    fn upgrades_old_database_keeping_data() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("old.db");
        {
            // v1 のままの DB を作る
            let c = Connection::open(&p).unwrap();
            c.execute_batch(MIGRATIONS[0]).unwrap();
            c.pragma_update(None, "user_version", 1).unwrap();
            c.execute(
                "INSERT INTO sessions(format, theme, started_at) VALUES ('mixed', '古いデータ', '2026-11-20T00:00:00Z')",
                [],
            )
            .unwrap();
        }
        let c = open(&p).unwrap();
        assert_eq!(schema_version(&c).unwrap(), MIGRATIONS.len() as i64);
        let theme: String = c.query_row("SELECT theme FROM sessions", [], |r| r.get(0)).unwrap();
        assert_eq!(theme, "古いデータ");
        let n: i64 = c.query_row("SELECT count(*) FROM diagram_adjustments", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn data_version_changes_on_other_connection_write() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m.db");
        let a = open(&p).unwrap();
        let b = open(&p).unwrap();
        let before = data_version(&a).unwrap();
        b.execute(
            "INSERT INTO sessions(format, theme, started_at) VALUES ('mixed', 't', '2026-11-20T00:00:00Z')",
            [],
        )
        .unwrap();
        assert_ne!(before, data_version(&a).unwrap());
    }
}
