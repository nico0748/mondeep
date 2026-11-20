//! 用語の表記ゆれを吸収するための正規化。
//!
//! 見出し語と別表記をすべて正規化した「照合キー」にして保存し、同じ用語の重複を防ぐ。
use unicode_normalization::UnicodeNormalization;

/// 照合キーを作る。
///
/// - NFKC で全角英数・半角カナをそろえる
/// - 英字は小文字にする
/// - ひらがなはカタカナにそろえる
/// - 空白・中黒・ハイフン・長音以外の記号（`-` `_` `/` `.` `・`）を取り除く
pub fn term_key(s: &str) -> String {
    s.nfkc()
        .flat_map(|c| c.to_lowercase())
        .filter_map(|c| {
            if c.is_whitespace() || matches!(c, '-' | '_' | '/' | '.' | '・' | '･' | '(' | ')' | '（' | '）') {
                None
            } else if ('\u{3041}'..='\u{3096}').contains(&c) {
                // ひらがな → カタカナ
                char::from_u32(c as u32 + 0x60)
            } else {
                Some(c)
            }
        })
        .collect()
}

/// 2 つの照合キーが「迷うほど近い」かどうか。完全一致は呼び出し側で先に判定する。
pub fn is_similar(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let (short, long) = if a.chars().count() <= b.chars().count() {
        (a, b)
    } else {
        (b, a)
    };
    // 片方がもう片方を含む（例: "tls" と "tls13"、"クロスサイトスクリプティング" と "xss" は別）
    if short.chars().count() >= 3 && long.contains(short) {
        return true;
    }
    strsim::normalized_levenshtein(a, b) >= 0.8
}

/// 表示幅の目安（全角 = 2、半角 = 1）。図解の文字はみ出し検査に使う。
pub fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| if c.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&c) { 1 } else { 2 })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_width_and_kana() {
        assert_eq!(term_key("TLS"), "tls");
        assert_eq!(term_key("ＴＬＳ"), "tls");
        assert_eq!(term_key("Cross-Site Scripting"), "crosssitescripting");
        assert_eq!(term_key("くろすさいと"), "クロスサイト");
        assert_eq!(term_key("ｸﾛｽｻｲﾄ"), "クロスサイト");
        assert_eq!(term_key("公開鍵 暗号"), "公開鍵暗号");
        assert_eq!(term_key("TLS ハンドシェイク"), term_key("tls・ハンドシェイク"));
    }

    #[test]
    fn similarity() {
        assert!(is_similar("tls", "tls13"));
        assert!(is_similar("ハンドシェイク", "ハンドシェーク"));
        assert!(!is_similar("xss", "csrf"));
        assert!(!is_similar("ip", "ipsec")); // 3 文字未満の部分一致は迷わない
    }

    #[test]
    fn width() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("暗号"), 4);
        assert_eq!(display_width("TLS 1.3"), 7);
    }
}
