//! 復習スケジュール（SM-2 系の簡易版）。
//!
//! 復習状態は「用語 × 形式」ごとに持つ。用語を答えられることと、意味を説明できることは別の力なので。
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::{Format, Verdict};

pub const INITIAL_EASE: f64 = 2.5;
pub const MIN_EASE: f64 = 1.3;
pub const MAX_INTERVAL_DAYS: i64 = 365;

/// スケジュール計算に使う状態（DB の review_states の中身）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    pub interval_days: i64,
    pub ease: f64,
    pub streak: i64,
    pub due_date: NaiveDate,
}

impl Schedule {
    pub fn new(today: NaiveDate) -> Self {
        Schedule {
            interval_days: 0,
            ease: INITIAL_EASE,
            streak: 0,
            due_date: today,
        }
    }
}

/// 1 回の復習の評価（SM-2 の quality、0〜5）を判定から決める。
///
/// - 意味→用語: 正解 5 / 部分正解 3 / 不正解 1
/// - 用語→意味＋深掘り: 「最初の問いの判定」と「正解できた最も深い段」で決める。
///   最初の問いが 正解 4 / 部分正解 3 / 不正解 1 を基本に、2 段目以上まで正解できたら +1（最大 5）。
///   最初の問いが不正解でも、深い段で正解できていれば 2 にとどめる（全く分かっていないわけではない）。
pub fn quality(format: Format, root_verdict: Verdict, max_correct_depth: Option<i64>) -> u8 {
    match format {
        Format::MeaningToTerm => match root_verdict {
            Verdict::Correct => 5,
            Verdict::Partial => 3,
            Verdict::Incorrect => 1,
        },
        Format::TermToMeaning => {
            let deep = max_correct_depth.unwrap_or(-1);
            match root_verdict {
                Verdict::Correct => if deep >= 2 { 5 } else { 4 },
                Verdict::Partial => if deep >= 2 { 4 } else { 3 },
                Verdict::Incorrect => if deep >= 1 { 2 } else { 1 },
            }
        }
    }
}

/// SM-2 の簡易版で次回の状態を計算する。
///
/// - quality < 3（不正解扱い）: 連続正解を 0 に戻し、翌日に復習
/// - quality >= 3: 連続正解 1 回目は 1 日後、2 回目は 3 日後、以降は 前回間隔 × 難易度係数
/// - 難易度係数は SM-2 の式で更新し、1.3 を下限にする
pub fn next(prev: Schedule, q: u8, today: NaiveDate) -> Schedule {
    let q = q.min(5) as f64;
    let ease = (prev.ease + 0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02)).max(MIN_EASE);
    let (streak, interval) = if q < 3.0 {
        (0, 1)
    } else {
        let streak = prev.streak + 1;
        let interval = match streak {
            1 => 1,
            2 => 3,
            _ => ((prev.interval_days.max(1) as f64) * ease).round() as i64,
        };
        (streak, interval.clamp(1, MAX_INTERVAL_DAYS))
    };
    Schedule {
        interval_days: interval,
        ease,
        streak,
        due_date: today + Duration::days(interval),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn correct_answers_grow_interval() {
        let today = d("2026-11-20");
        let s0 = Schedule::new(today);
        let s1 = next(s0, 5, today);
        assert_eq!((s1.streak, s1.interval_days), (1, 1));
        assert_eq!(s1.due_date, d("2026-11-21"));
        let s2 = next(s1, 5, d("2026-11-21"));
        assert_eq!((s2.streak, s2.interval_days), (2, 3));
        let s3 = next(s2, 5, d("2026-11-24"));
        assert_eq!(s3.streak, 3);
        // 3 × (2.5 + 0.1 + 0.1 + 0.1) ≒ 8
        assert_eq!(s3.interval_days, 8);
        assert!(s3.ease > INITIAL_EASE);
    }

    #[test]
    fn incorrect_resets_streak_and_lowers_ease() {
        let today = d("2026-11-20");
        let mut s = Schedule::new(today);
        for _ in 0..3 {
            s = next(s, 5, today);
        }
        let failed = next(s, 1, today);
        assert_eq!(failed.streak, 0);
        assert_eq!(failed.interval_days, 1);
        assert!(failed.ease < s.ease);
        assert_eq!(failed.due_date, d("2026-11-21"));
    }

    #[test]
    fn ease_has_floor() {
        let today = d("2026-11-20");
        let mut s = Schedule::new(today);
        for _ in 0..20 {
            s = next(s, 0, today);
        }
        assert!((s.ease - MIN_EASE).abs() < 1e-9);
    }

    #[test]
    fn partial_passes_but_lowers_ease() {
        let today = d("2026-11-20");
        let s = next(Schedule::new(today), 3, today);
        assert_eq!(s.streak, 1);
        assert!(s.ease < INITIAL_EASE);
    }

    #[test]
    fn interval_is_capped() {
        let today = d("2026-11-20");
        let s = Schedule { interval_days: 300, ease: 2.5, streak: 10, due_date: today };
        assert_eq!(next(s, 5, today).interval_days, MAX_INTERVAL_DAYS);
    }

    #[test]
    fn quality_for_deep_dive() {
        use Verdict::*;
        assert_eq!(quality(Format::MeaningToTerm, Correct, None), 5);
        assert_eq!(quality(Format::MeaningToTerm, Partial, None), 3);
        assert_eq!(quality(Format::TermToMeaning, Correct, Some(0)), 4);
        assert_eq!(quality(Format::TermToMeaning, Correct, Some(2)), 5);
        assert_eq!(quality(Format::TermToMeaning, Partial, Some(3)), 4);
        assert_eq!(quality(Format::TermToMeaning, Incorrect, None), 1);
        assert_eq!(quality(Format::TermToMeaning, Incorrect, Some(1)), 2);
    }
}
