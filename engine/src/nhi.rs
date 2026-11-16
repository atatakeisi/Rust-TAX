//! 国民健康保険料（40歳未満・介護分なし）
//!
//! 保険料 = 所得割（旧ただし書き所得 × 料率）＋ 均等割 ＋ 平等割 ＋ 子ども・子育て支援金分
//!
//! - 旧ただし書き所得 = 総所得金額等 − 43万円
//! - 前年の所得で当年度（4月〜翌3月）の保険料が決まる
//!   （2025年の所得 → 令和8年度、2026年の所得 → 令和9年度）
//! - 低所得世帯は均等割・平等割を 7割/5割/2割 軽減（令和8年度基準）
//!   - 7割: 総所得金額等 ≤ 43万 ＋ 10万×(給与所得者等の数−1)
//!   - 5割: ≤ 43万 ＋ 31万×被保険者数 ＋ 10万×(給与所得者等の数−1)
//!   - 2割: ≤ 43万 ＋ 57万×被保険者数 ＋ 10万×(給与所得者等の数−1)
//! - 親と同居で親が世帯主（擬制世帯主）の場合、親の所得も軽減判定に含まれるため、
//!   本エンジンでは「軽減なし」として保守的に計算する。

use crate::prefectures::{self, KODOMO_PER_CAPITA, KODOMO_RATE_BP, NHI_CAP};
use crate::util::{floor_to, MAN};

const NHI_BASIC_DEDUCTION: i64 = 43 * MAN;

/// 計算結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NhiCalc {
    /// 年額保険料
    pub premium: i64,
    /// 軽減割合（0/2/5/7割）
    pub reduction: u8,
}

/// 軽減割合の判定（被保険者1人の世帯）
pub fn reduction_rate(total_income: i64, salary_earners: u32, insured: u32) -> u8 {
    let extra = 10 * MAN * (salary_earners.max(1) as i64 - 1);
    let n = insured.max(1) as i64;
    if total_income <= NHI_BASIC_DEDUCTION + extra {
        7
    } else if total_income <= NHI_BASIC_DEDUCTION + 31 * MAN * n + extra {
        5
    } else if total_income <= NHI_BASIC_DEDUCTION + 57 * MAN * n + extra {
        2
    } else {
        0
    }
}

/// 国保料（年額）を計算
///
/// * `prefecture` - 都道府県インデックス
/// * `total_income` - 総所得金額等（前年分）
/// * `age` - 年度中の年齢（子ども・子育て支援金分の均等割判定に使用）
/// * `lives_alone` - 一人暮らし（自分が世帯主）か
/// * `has_salary` - 給与所得者か
pub fn calc(prefecture: usize, total_income: i64, age: u32, lives_alone: bool, has_salary: bool) -> NhiCalc {
    let rate = prefectures::get(prefecture);
    let base = (total_income - NHI_BASIC_DEDUCTION).max(0);

    let reduction = if lives_alone {
        reduction_rate(total_income.max(0), if has_salary { 1 } else { 0 }, 1)
    } else {
        0
    };
    let keep = |amount: i64| amount * (10 - reduction as i64) / 10;

    let income_levy = base * (rate.income_rate_bp + KODOMO_RATE_BP) / 10_000;
    let kodomo_capita = if age >= 18 { KODOMO_PER_CAPITA } else { 0 };
    let flat = keep(rate.per_capita) + keep(rate.per_household) + keep(kodomo_capita);

    let premium = floor_to((income_levy + flat).min(NHI_CAP), 100);
    NhiCalc { premium, reduction }
}

/// 国保の年度ラベル（前年所得 → 当年度）
pub fn fiscal_label(income_year: u32) -> String {
    let fy = income_year + 1;
    format!("令和{}年度（{}年4月〜{}年3月）", fy - 2018, fy, fy + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefectures::TOKYO_INDEX;

    #[test]
    fn reduction_thresholds() {
        assert_eq!(reduction_rate(43 * MAN, 0, 1), 7);
        assert_eq!(reduction_rate(74 * MAN, 0, 1), 5);
        assert_eq!(reduction_rate(74 * MAN + 1, 0, 1), 2);
        assert_eq!(reduction_rate(100 * MAN, 0, 1), 2);
        assert_eq!(reduction_rate(100 * MAN + 1, 0, 1), 0);
    }

    #[test]
    fn tokyo_with_parents_no_reduction() {
        // 所得50万・23区・同居 → (50-43)万×10.58% = 7,406 + 65,200 + 1,900 = 74,506 → 74,500
        let c = calc(TOKYO_INDEX, 50 * MAN, 20, false, false);
        assert_eq!(c.reduction, 0);
        assert_eq!(c.premium, 74_500);
    }

    #[test]
    fn tokyo_alone_gets_reduction() {
        // 一人暮らし・所得50万 → 5割軽減: 7,406 + 32,600 + 950 = 40,956 → 40,900
        let c = calc(TOKYO_INDEX, 50 * MAN, 20, true, false);
        assert_eq!(c.reduction, 5);
        assert_eq!(c.premium, 40_900);
    }

    #[test]
    fn cap_applies() {
        let c = calc(TOKYO_INDEX, 20_000 * MAN, 25, true, false);
        assert_eq!(c.premium, NHI_CAP);
    }

    #[test]
    fn minors_skip_kodomo_per_capita() {
        let adult = calc(TOKYO_INDEX, 0, 20, false, false).premium;
        let minor = calc(TOKYO_INDEX, 0, 17, false, false).premium;
        assert_eq!(adult - minor, 1_900);
    }
}
