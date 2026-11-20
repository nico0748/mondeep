//! 共通DB層の結合テスト（Claude が MCP で記録する流れを再現する）。
use chrono::NaiveDate;
use mondeep_core::model::*;
use mondeep_core::store::{RecordAttempt, RecordQuestion, TermInput};
use mondeep_core::{Error, Store};
use serde_json::json;

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn term(h: &str, aliases: &[&str], tags: &[&str]) -> TermInput {
    TermInput {
        headword: h.into(),
        aliases: aliases.iter().map(|s| s.to_string()).collect(),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

fn attempt(qid: i64, v: Verdict) -> RecordAttempt {
    RecordAttempt {
        question_id: qid,
        answer: "回答".into(),
        verdict: v,
        reason: "理由".into(),
        explanation_md: Some("## 解説\n本文".into()),
        diagrams: vec![],
        sources: vec![],
    }
}

fn ask(store: &Store, sid: i64, t: TermInput, parent: Option<i64>, format: Option<Format>) -> Question {
    store
        .record_question(&RecordQuestion {
            session_id: sid,
            format,
            prompt: format!("{} について説明してください", t.headword),
            model_answer: None,
            term: if parent.is_some() && t.headword.is_empty() { None } else { Some(t) },
            parent_id: parent,
        })
        .unwrap()
}

#[test]
fn term_matching_merges_aliases_and_detects_ambiguity() {
    let s = Store::open_in_memory().unwrap();
    let xss = s.resolve_term(&term("クロスサイトスクリプティング", &["XSS"], &["Web"])).unwrap();
    // 別表記の大文字小文字・全角は同じ用語
    let again = s.resolve_term(&term("ｘｓｓ", &["Cross Site Scripting"], &["面接"])).unwrap();
    assert_eq!(xss.id, again.id);
    assert!(again.aliases.contains(&"Cross Site Scripting".to_string()));
    assert_eq!(again.tags, vec!["Web", "面接"]);
    // ひらがな表記も一致
    assert_eq!(s.find_term("くろすさいとすくりぷてぃんぐ").unwrap().unwrap().id, xss.id);

    // 近いが一致しない → 候補を返す
    let tls = s.resolve_term(&term("TLS", &[], &[])).unwrap();
    match s.resolve_term(&term("TLS 1.3", &[], &[])) {
        Err(Error::AmbiguousTerm { candidates, .. }) => assert_eq!(candidates[0].term_id, tls.id),
        other => panic!("expected ambiguity, got {other:?}"),
    }
    // create_new で新規登録、term_id で既存指定
    let mut t13 = term("TLS 1.3", &[], &[]);
    t13.create_new = true;
    assert_ne!(s.resolve_term(&t13).unwrap().id, tls.id);
    let mut pick = term("Transport Layer Security", &[], &[]);
    pick.term_id = Some(tls.id);
    let tls2 = s.resolve_term(&pick).unwrap();
    assert_eq!(tls2.id, tls.id);
    assert!(tls2.aliases.contains(&"Transport Layer Security".to_string()));

    // 2 つの既存用語に一致 → 候補を返す
    match s.resolve_term(&term("XSS", &["TLS"], &[])) {
        Err(Error::AmbiguousTerm { candidates, .. }) => assert_eq!(candidates.len(), 2),
        other => panic!("expected ambiguity, got {other:?}"),
    }
}

#[test]
fn deep_dive_tree_and_review_schedule() {
    let s = Store::open_in_memory().unwrap();
    let today = d("2026-11-20");
    let sess = s.start_session(SessionFormat::TermToMeaning, "XSS で面接形式の深掘り").unwrap();
    let root = ask(&s, sess.id, term("XSS", &[], &["Web"]), None, None);
    assert_eq!((root.depth, root.format), (0, Format::TermToMeaning));
    let r = s.record_attempt(&attempt(root.id, Verdict::Correct), today).unwrap();
    // 根が正解（深掘りなし）→ quality 4 → 1 日後
    assert_eq!(r.review.due_date, "2026-11-21");
    assert_eq!(r.review.streak, 1);

    let child = ask(&s, sess.id, term("", &[], &[]), Some(root.id), None);
    assert_eq!((child.depth, child.term_id), (1, root.term_id));
    s.record_attempt(&attempt(child.id, Verdict::Correct), today).unwrap();
    let grand = ask(&s, sess.id, term("", &[], &[]), Some(child.id), None);
    assert_eq!(grand.depth, 2);
    let r = s.record_attempt(&attempt(grand.id, Verdict::Correct), today).unwrap();
    // 同じ木の回答は 1 回の復習として数える（streak は 1 のまま、quality 5 で計算し直す）
    assert_eq!(r.review.streak, 1);
    assert!(r.review.ease > 2.5);

    let detail = s.question_detail(grand.id).unwrap();
    assert_eq!(detail.root_id, root.id);
    assert_eq!(detail.tree.len(), 3);
    assert_eq!(detail.attempts.len(), 1);
    assert!(detail.explanation.is_some());

    // 次の日の新しい木は 2 回目の復習
    let sess2 = s.start_session(SessionFormat::TermToMeaning, "今日の復習").unwrap();
    let root2 = ask(&s, sess2.id, term("xss", &[], &[]), None, None);
    let r = s.record_attempt(&attempt(root2.id, Verdict::Correct), d("2026-11-21")).unwrap();
    assert_eq!(r.review.streak, 2);
    assert_eq!(r.review.due_date, "2026-11-24");

    let end = s.end_session(sess.id, "よくできた").unwrap();
    assert_eq!(end.accuracy, Some(1.0));
    assert_eq!(end.question_count, 3);
}

#[test]
fn child_must_be_in_same_session() {
    let s = Store::open_in_memory().unwrap();
    let a = s.start_session(SessionFormat::TermToMeaning, "a").unwrap();
    let b = s.start_session(SessionFormat::TermToMeaning, "b").unwrap();
    let q = ask(&s, a.id, term("CSRF", &[], &[]), None, None);
    let err = s
        .record_question(&RecordQuestion {
            session_id: b.id,
            prompt: "深掘り".into(),
            parent_id: Some(q.id),
            ..Default::default()
        })
        .unwrap_err();
    assert!(err.to_string().contains("別のセッション"));
    // 混合セッションは format 必須
    let m = s.start_session(SessionFormat::Mixed, "m").unwrap();
    let err = s
        .record_question(&RecordQuestion {
            session_id: m.id,
            prompt: "p".into(),
            term: Some(term("CSRF", &[], &[])),
            ..Default::default()
        })
        .unwrap_err();
    assert!(err.to_string().contains("format"));
}

#[test]
fn diagrams_are_validated_before_saving() {
    let s = Store::open_in_memory().unwrap();
    let sess = s.start_session(SessionFormat::MeaningToTerm, "TLS").unwrap();
    let q = ask(&s, sess.id, term("TLS", &[], &[]), None, None);
    let mut a = attempt(q.id, Verdict::Correct);
    a.diagrams = vec![json!({"version": 1, "kind": "concept", "nodes": [
        {"id": "a", "label": "A", "row": 0, "col": 0},
        {"id": "b", "label": "B", "row": 0, "col": 1}
    ], "edges": [{"from": "a", "to": "b", "label": ""}]})];
    match s.record_attempt(&a, d("2026-11-20")) {
        Err(Error::Diagram(errs)) => assert!(errs[0].contains("ラベル")),
        other => panic!("expected diagram error, got {other:?}"),
    }
    // 何も保存されていない
    assert!(s.question_detail(q.id).unwrap().attempts.is_empty());

    a.diagrams[0]["edges"][0]["label"] = json!("暗号化通信");
    a.sources = vec![Source { kind: SourceKind::Web, name: "RFC 8446".into(), url: Some("https://www.rfc-editor.org/rfc/rfc8446".into()) }];
    let r = s.record_attempt(&a, d("2026-11-20")).unwrap();
    assert!(r.explanation_id.is_some());
    let e = s.question_detail(q.id).unwrap().explanation.unwrap();
    assert_eq!(e.diagrams.len(), 1);
    assert_eq!(e.sources[0].name, "RFC 8446");

    a.sources = vec![Source { kind: SourceKind::Web, name: "x".into(), url: Some("javascript:alert(1)".into()) }];
    assert!(s.record_attempt(&a, d("2026-11-20")).is_err());
}

#[test]
fn review_queue_weak_terms_and_flags() {
    let s = Store::open_in_memory().unwrap();
    let today = d("2026-11-20");
    let sess = s.start_session(SessionFormat::MeaningToTerm, "暗号").unwrap();
    for (h, v) in [("AES", Verdict::Correct), ("RSA", Verdict::Incorrect), ("ECDH", Verdict::Partial)] {
        let q = ask(&s, sess.id, term(h, &[], &["暗号"]), None, None);
        s.record_attempt(&attempt(q.id, v), today).unwrap();
    }
    // 今日の時点では何も期日が来ていない
    assert!(s.review_queue(10, &[], today).unwrap().is_empty());
    // 翌日は全部
    let q = s.review_queue(10, &[], d("2026-11-21")).unwrap();
    assert_eq!(q.len(), 3);
    assert!(q[0].summary.contains("過去 1 問"));

    // 要復習の印は期日前でも出る（先頭）
    let aes = s.find_term("AES").unwrap().unwrap();
    s.set_needs_review(aes.id, true).unwrap();
    let q = s.review_queue(10, &[], today).unwrap();
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].headword, "AES");
    assert!(q[0].summary.contains("要復習"));
    // タグで絞り込み
    assert!(s.review_queue(10, &["Web".into()], today).unwrap().is_empty());

    let weak = s.weak_terms(10, &[]).unwrap();
    let names: Vec<_> = weak.iter().map(|w| w.headword.as_str()).collect();
    assert_eq!(names[0], "RSA");
    assert!(names.contains(&"ECDH"));
    assert!(names.contains(&"AES")); // 印あり
    assert!(weak[0].reasons[0].contains("不正解 1"));
}

#[test]
fn shallow_deep_dive_is_weak() {
    let s = Store::open_in_memory().unwrap();
    let today = d("2026-11-20");
    let sess = s.start_session(SessionFormat::TermToMeaning, "PKI").unwrap();
    let root = ask(&s, sess.id, term("PKI", &[], &[]), None, None);
    s.record_attempt(&attempt(root.id, Verdict::Correct), today).unwrap();
    let child = ask(&s, sess.id, term("", &[], &[]), Some(root.id), None);
    s.record_attempt(&attempt(child.id, Verdict::Incorrect), today).unwrap();
    let weak = s.weak_terms(10, &[]).unwrap();
    assert_eq!(weak.len(), 1);
    assert_eq!(weak[0].avg_max_correct_depth, Some(0.0));
    assert!(weak.iter().any(|w| w.reasons.iter().any(|r| r.contains("浅い段"))));
    let card = s.term_card(root.term_id).unwrap();
    assert_eq!(card.max_correct_depth, Some(0));
    assert_eq!(card.questions.len(), 2);
}

#[test]
fn history_filters_and_search() {
    let s = Store::open_in_memory().unwrap();
    let today = d("2026-11-20");
    let a = s.start_session(SessionFormat::MeaningToTerm, "Web").unwrap();
    let q1 = ask(&s, a.id, term("SQL インジェクション", &["SQLi"], &["Web"]), None, None);
    s.record_attempt(&attempt(q1.id, Verdict::Incorrect), today).unwrap();
    let b = s.start_session(SessionFormat::TermToMeaning, "ネットワーク").unwrap();
    let q2 = ask(&s, b.id, term("ARP スプーフィング", &[], &["ネットワーク"]), None, None);
    let mut at = attempt(q2.id, Verdict::Correct);
    at.explanation_md = Some("MAC アドレスを偽装してなりすます攻撃".into());
    s.record_attempt(&at, today).unwrap();

    let all = s.history(&HistoryFilter::default()).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].session.session.id, b.id); // 新しい順
    let web = s.history(&HistoryFilter { tag: Some("Web".into()), ..Default::default() }).unwrap();
    assert_eq!(web.len(), 1);
    assert_eq!(web[0].session.incorrect, 1);
    let wrong = s.history(&HistoryFilter { verdict: Some(Verdict::Correct), ..Default::default() }).unwrap();
    assert_eq!(wrong.len(), 1);
    assert_eq!(wrong[0].questions[0].question.id, q2.id);
    let fmt = s.history(&HistoryFilter { format: Some(Format::MeaningToTerm), ..Default::default() }).unwrap();
    assert_eq!(fmt[0].questions[0].question.id, q1.id);

    // 日本語の部分一致（3 文字以上は trigram 索引、2 文字以下も LIKE で引ける）
    let hits = s.search("なりすま", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, "explanation");
    assert_eq!(hits[0].question_id, Some(q2.id));
    assert!(!s.search("MAC", 10).unwrap().is_empty());
    assert_eq!(s.search("sqli", 10).unwrap()[0].kind, "term");
    assert!(!s.search("偽装", 10).unwrap().is_empty());
    assert!(s.search("100%", 10).unwrap().is_empty());

    // セッション削除（アプリから）で問題も消える
    s.delete_session(a.id).unwrap();
    assert!(s.search("sql イン", 10).unwrap().iter().all(|h| h.kind == "term"));
    assert_eq!(s.history(&HistoryFilter::default()).unwrap().len(), 1);
}

#[test]
fn ten_thousand_questions_stay_fast() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(&dir.path().join("perf.db")).unwrap();
    let today = d("2026-11-20");
    s.conn().execute_batch("BEGIN").unwrap();
    let mut sid = 0;
    for i in 0..10_000 {
        if i % 20 == 0 {
            sid = s.start_session(SessionFormat::MeaningToTerm, &format!("session {i}")).unwrap().id;
        }
        let mut t = term(&format!("用語{}", i % 500), &[], &["Web"]);
        t.create_new = true;
        let t = match s.find_term(&t.headword).unwrap() {
            Some(x) => TermInput { term_id: Some(x.id), ..t },
            None => t,
        };
        let q = ask(&s, sid, t, None, None);
        s.record_attempt(&attempt(q.id, if i % 3 == 0 { Verdict::Incorrect } else { Verdict::Correct }), today).unwrap();
    }
    s.conn().execute_batch("COMMIT").unwrap();

    let t0 = std::time::Instant::now();
    let h = s.history(&HistoryFilter { tag: Some("Web".into()), ..Default::default() }).unwrap();
    assert_eq!(h.len(), 50);
    let hits = s.search("説明して", 100).unwrap();
    assert_eq!(hits.len(), 100);
    s.weak_terms(20, &[]).unwrap();
    s.list_terms(None, None).unwrap();
    let elapsed = t0.elapsed();
    assert!(elapsed.as_millis() < 1000 * if cfg!(debug_assertions) { 5 } else { 1 }, "{elapsed:?}");
}
