//! 住民税（所得割 10% ＋ 均等割 5,000円）
//!
//! - 基礎控除 43万円（2025・2026年分とも）
//! - 非課税ライン: 合計所得 45万円以下（単身・1級地基準）
//! - 未成年（1/1時点で18歳未満）は合計所得 135万円以下なら非課税
//! - 均等割 5,000円 = 市区町村3,000円＋道府県1,000円＋森林環境税1,000円
//! - 2025年の所得 → 令和8年度住民税（2026年6月〜）、2026年の所得 → 令和9年度（2027年6月〜）

use crate::util::{floor_to, MAN};

pub const BASIC_DEDUCTION: i64 = 43 * MAN;
pub const EXEMPT_LINE: i64 = 45 * MAN;
pub const MINOR_EXEMPT_LINE: i64 = 135 * MAN;
pub const PER_CAPITA: i64 = 5_000;
/// 単身者の人的控除差（基礎控除 所得税48万 − 住民税43万 ＝ 5万円で固定）
const PERSONAL_DEDUCTION_GAP: i64 = 5 * MAN;

/// 住民税の年額
///
/// * `total_income` - 合計所得金額（住民税用の給与所得で計算したもの）
/// * `social_deduction` - 社会保険料控除
/// * `age_at_jan1` - 賦課期日（翌年1月1日）時点の年齢
pub fn calc(total_income: i64, social_deduction: i64, age_at_jan1: u32) -> i64 {
    if total_income <= EXEMPT_LINE {
        return 0;
    }
    if age_at_jan1 < 18 && total_income <= MINOR_EXEMPT_LINE {
        return 0;
    }
    let taxable = floor_to(total_income - BASIC_DEDUCTION - social_deduction.max(0), 1_000);
    let mut income_levy = 0;
    if taxable > 0 {
        // 調整控除（単身者）
        let adjust = if taxable <= 200 * MAN {
            taxable.min(PERSONAL_DEDUCTION_GAP) * 5 / 100
        } else {
            ((PERSONAL_DEDUCTION_GAP - (taxable - 200 * MAN)) * 5 / 100).max(2_500)
        };
        income_levy = floor_to(taxable * 10 / 100 - adjust, 100);
    }
    income_levy + PER_CAPITA
}

/// 住民税年度のラベル
pub fn fiscal_label(income_year: u32) -> String {
    let fy = income_year + 1;
    format!("令和{}年度（{}年6月〜）", fy - 2018, fy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exempt_lines() {
        assert_eq!(calc(45 * MAN, 0, 20), 0);
        // 課税所得2.1万 → 所得割 2,100 - 調整控除1,050 = 1,050 → 1,000（100円未満切捨て）
        assert_eq!(calc(45 * MAN + 1_000, 0, 20), 1_000 + 5_000);
    }

    #[test]
    fn small_income() {
        // 合計所得 50万: 課税7万 → 7,000 - 2,500 = 4,500 + 5,000
        assert_eq!(calc(50 * MAN, 0, 20), 9_500);
    }

    #[test]
    fn minor_exemption() {
        assert_eq!(calc(100 * MAN, 0, 17), 0);
        assert!(calc(100 * MAN, 0, 18) > 0);
        assert!(calc(140 * MAN, 0, 17) > 0);
    }

    #[test]
    fn example_streamer() {
        // 所得220万・社保控除289,540 → 課税1,480,000 → 148,000 - 2,500 + 5,000
        assert_eq!(calc(2_200_000, 289_540, 20), 150_500);
    }

    #[test]
    fn labels() {
        assert_eq!(fiscal_label(2025), "令和8年度（2026年6月〜）");
        assert_eq!(fiscal_label(2026), "令和9年度（2027年6月〜）");
    }
}
