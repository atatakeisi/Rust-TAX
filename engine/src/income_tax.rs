//! 所得税（＋復興特別所得税）
//!
//! - 2025年（令和7年）分: 令和7年度税制改正後。基礎控除は合計所得に応じて 95/88/68/63/58万円…
//!   給与所得控除の最低保障額 65万円。
//! - 2026年（令和8年）分: 令和8年度税制改正後。基礎控除 本則62万円＋特例加算（令和8・9年分）で
//!   合計所得489万円以下は最大104万円。給与所得控除の最低保障額 74万円（本則69万＋特例5万）。

use crate::util::{floor_to, MAN};

/// 所得税の基礎控除（年度別・合計所得金額に応じた段階）
pub fn basic_deduction(year: u32, total_income: i64) -> i64 {
    let t = total_income;
    if year >= 2026 {
        // 令和8年分〜（本則62万＋特例加算42万/5万）
        match t {
            _ if t <= 489 * MAN => 104 * MAN,
            _ if t <= 655 * MAN => 67 * MAN,
            _ if t <= 2_350 * MAN => 62 * MAN,
            _ if t <= 2_400 * MAN => 48 * MAN,
            _ if t <= 2_450 * MAN => 32 * MAN,
            _ if t <= 2_500 * MAN => 16 * MAN,
            _ => 0,
        }
    } else {
        // 令和7年分（本則58万＋特例加算37万/30万/10万/5万）
        match t {
            _ if t <= 132 * MAN => 95 * MAN,
            _ if t <= 336 * MAN => 88 * MAN,
            _ if t <= 489 * MAN => 68 * MAN,
            _ if t <= 655 * MAN => 63 * MAN,
            _ if t <= 2_350 * MAN => 58 * MAN,
            _ if t <= 2_400 * MAN => 48 * MAN,
            _ if t <= 2_450 * MAN => 32 * MAN,
            _ if t <= 2_500 * MAN => 16 * MAN,
            _ => 0,
        }
    }
}

/// 給与所得控除の額。`min_deduction` は最低保障額
fn salary_deduction(salary: i64, min_deduction: i64) -> i64 {
    let s = salary.max(0);
    let formula = match s {
        _ if s <= 190 * MAN => 0,
        _ if s <= 360 * MAN => s * 30 / 100 + 8 * MAN,
        _ if s <= 660 * MAN => s * 20 / 100 + 44 * MAN,
        _ if s <= 850 * MAN => s * 10 / 100 + 110 * MAN,
        _ => 195 * MAN,
    };
    formula.max(min_deduction).min(s)
}

/// 給与所得（所得税用）
pub fn salary_income(year: u32, salary: i64) -> i64 {
    let min = if year >= 2026 { 74 * MAN } else { 65 * MAN };
    (salary - salary_deduction(salary, min)).max(0)
}

/// 給与所得（住民税用）。住民税は令和8年度税制改正の特例5万円の上乗せなし（本則のみ）
pub fn salary_income_for_resident(year: u32, salary: i64) -> i64 {
    let min = if year >= 2026 { 69 * MAN } else { 65 * MAN };
    (salary - salary_deduction(salary, min)).max(0)
}

/// 課税所得金額（1,000円未満切り捨て）に対する所得税＋復興特別所得税（100円未満切り捨て）
pub fn tax_on_taxable_income(taxable: i64) -> i64 {
    let t = floor_to(taxable, 1_000);
    if t <= 0 {
        return 0;
    }
    // (上限, 税率%, 控除額)
    const BRACKETS: [(i64, i64, i64); 7] = [
        (1_950_000, 5, 0),
        (3_300_000, 10, 97_500),
        (6_950_000, 20, 427_500),
        (9_000_000, 23, 636_000),
        (18_000_000, 33, 1_536_000),
        (40_000_000, 40, 2_796_000),
        (i64::MAX, 45, 4_796_000),
    ];
    let (_, rate, ded) = BRACKETS
        .iter()
        .copied()
        .find(|(upper, _, _)| t <= *upper)
        .unwrap_or((i64::MAX, 45, 4_796_000));
    let base = t * rate / 100 - ded;
    // 復興特別所得税 2.1% を加算（基準所得税額 × 102.1%）
    let with_reconstruction = base * 1_021 / 1_000;
    floor_to(with_reconstruction, 100)
}

/// 所得税の計算過程
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeTaxCalc {
    pub basic_deduction: i64,
    pub taxable_income: i64,
    pub tax: i64,
}

/// 合計所得と社会保険料控除から所得税を計算
pub fn calc(year: u32, total_income: i64, social_deduction: i64) -> IncomeTaxCalc {
    let basic = basic_deduction(year, total_income);
    let taxable = floor_to(total_income - basic - social_deduction.max(0), 1_000);
    IncomeTaxCalc {
        basic_deduction: basic,
        taxable_income: taxable,
        tax: tax_on_taxable_income(taxable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_deduction_2025_steps() {
        assert_eq!(basic_deduction(2025, 50 * MAN), 95 * MAN);
        assert_eq!(basic_deduction(2025, 132 * MAN), 95 * MAN);
        assert_eq!(basic_deduction(2025, 132 * MAN + 1), 88 * MAN);
        assert_eq!(basic_deduction(2025, 400 * MAN), 68 * MAN);
        assert_eq!(basic_deduction(2025, 600 * MAN), 63 * MAN);
        assert_eq!(basic_deduction(2025, 1_000 * MAN), 58 * MAN);
        assert_eq!(basic_deduction(2025, 3_000 * MAN), 0);
    }

    #[test]
    fn basic_deduction_2026_steps() {
        assert_eq!(basic_deduction(2026, 220 * MAN), 104 * MAN);
        assert_eq!(basic_deduction(2026, 489 * MAN), 104 * MAN);
        assert_eq!(basic_deduction(2026, 500 * MAN), 67 * MAN);
        assert_eq!(basic_deduction(2026, 1_000 * MAN), 62 * MAN);
    }

    #[test]
    fn salary_income_minimums() {
        // 2025: 給与160万 → 給与所得95万（最低保障65万）
        assert_eq!(salary_income(2025, 160 * MAN), 95 * MAN);
        // 2026: 給与178万 → 給与所得104万（最低保障74万）→ 基礎控除104万で所得税ゼロ＝178万の壁
        assert_eq!(salary_income(2026, 178 * MAN), 104 * MAN);
        assert_eq!(calc(2026, salary_income(2026, 178 * MAN), 0).tax, 0);
        // 給与が少なければ0
        assert_eq!(salary_income(2026, 50 * MAN), 0);
    }

    #[test]
    fn progressive_rates() {
        // 課税所得100万: 5% → 50,000 × 1.021 = 51,050
        assert_eq!(tax_on_taxable_income(1_000_000), 51_000);
        // 課税所得300万: 10% - 97,500 = 202,500 × 1.021 = 206,752 → 206,700
        assert_eq!(tax_on_taxable_income(3_000_000), 206_700);
        // 課税所得500万: 20% - 427,500 = 572,500 × 1.021 = 584,522 → 584,500
        assert_eq!(tax_on_taxable_income(5_000_000), 584_500);
        assert_eq!(tax_on_taxable_income(0), 0);
        assert_eq!(tax_on_taxable_income(999), 0);
    }

    #[test]
    fn example_streamer_2026() {
        // 所得220万・社保控除289,540円
        let c = calc(2026, 2_200_000, 289_540);
        assert_eq!(c.basic_deduction, 1_040_000);
        assert_eq!(c.taxable_income, 870_000);
        assert_eq!(c.tax, 44_400);
    }
}
