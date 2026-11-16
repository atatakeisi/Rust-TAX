//! 国民年金（第1号被保険者）
//!
//! - 令和7年度 月額 17,510円 / 令和8年度 月額 17,920円
//! - 20歳以上60歳未満で、会社の厚生年金に入っていない人が対象
//! - 本エンジンでは簡略化のため「その年の年齢が20歳以上なら12か月分」を計上する

use crate::util::MAN;

pub const MONTHLY_2025: i64 = 17_510;
pub const MONTHLY_2026: i64 = 17_920;

/// 全額免除の所得基準（単身: (扶養0+1)×35万＋32万）
pub const FULL_EXEMPTION_LINE: i64 = 67 * MAN;
/// 学生納付特例の所得基準（単身）
pub const STUDENT_DEFERMENT_LINE: i64 = 128 * MAN;

/// 年額
pub fn annual(year: u32, age_in_year: u32, has_shaho: bool) -> i64 {
    if age_in_year < 20 || age_in_year >= 60 || has_shaho {
        return 0;
    }
    let monthly = if year >= 2026 { MONTHLY_2026 } else { MONTHLY_2025 };
    monthly * 12
}

/// 免除・猶予の案内文（前年所得ベース）
pub fn exemption_hint(prev_year_income: i64, age: u32) -> String {
    if (19..=22).contains(&age) && prev_year_income <= STUDENT_DEFERMENT_LINE {
        format!(
            "学生なら「学生納付特例」で支払いを後回しにできる可能性あり（前年所得{}万円以下が目安）。役所の年金窓口かマイナポータルで申請。",
            STUDENT_DEFERMENT_LINE / MAN
        )
    } else if prev_year_income <= FULL_EXEMPTION_LINE {
        format!(
            "前年の所得が{}万円以下なら「全額免除」になる可能性あり（本人・世帯主の所得で判定）。未納のまま放置せず、必ず免除申請を。",
            FULL_EXEMPTION_LINE / MAN
        )
    } else {
        "所得的に免除はむずかしいライン。1年前納（口座振替）にすると少し割引になる。払った年金は全額「社会保険料控除」で税金が安くなる。".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annual_amounts() {
        assert_eq!(annual(2025, 20, false), 210_120);
        assert_eq!(annual(2026, 20, false), 215_040);
        assert_eq!(annual(2026, 19, false), 0);
        assert_eq!(annual(2026, 25, true), 0);
    }
}
