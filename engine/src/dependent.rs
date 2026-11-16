//! 親の扶養判定（税法上の扶養・健康保険の扶養）

use crate::model::{DependentJudge, Status};
use crate::util::{fmt_yen, MAN};

/// 税法上の扶養の所得ライン（扶養親族の合計所得金額要件）
pub fn tax_line(year: u32) -> i64 {
    if year >= 2026 {
        62 * MAN
    } else {
        58 * MAN
    }
}

/// 特定親族特別控除の上限（19〜22歳）
pub const SPECIAL_RELATIVE_UPPER: i64 = 123 * MAN;

/// 親が受けられる控除額 (所得税, 住民税)
///
/// * 一般の扶養親族（16〜18歳・23歳以上）: 38万 / 33万
/// * 特定扶養親族（19〜22歳）: 63万 / 45万
/// * 19〜22歳でラインを超えて123万円以下: 特定親族特別控除（段階的に減る）
pub fn parent_deduction(year: u32, age: u32, total_income: i64) -> (i64, i64) {
    if age < 16 {
        return (0, 0);
    }
    let line = tax_line(year);
    let specific = (19..=22).contains(&age);
    if total_income <= line {
        return if specific { (63 * MAN, 45 * MAN) } else { (38 * MAN, 33 * MAN) };
    }
    if !specific {
        return (0, 0);
    }
    let t = total_income;
    let (it, rt) = match t {
        _ if t <= 85 * MAN => (63, 45),
        _ if t <= 90 * MAN => (61, 45),
        _ if t <= 95 * MAN => (51, 45),
        _ if t <= 100 * MAN => (41, 41),
        _ if t <= 105 * MAN => (31, 31),
        _ if t <= 110 * MAN => (21, 21),
        _ if t <= 115 * MAN => (11, 11),
        _ if t <= 120 * MAN => (6, 6),
        _ if t <= SPECIAL_RELATIVE_UPPER => (3, 3),
        _ => (0, 0),
    };
    (it * MAN, rt * MAN)
}

/// 控除が減ったことによる親の税金の増加額（親の所得税率10%＋住民税10%を想定）
fn parent_increase(full: (i64, i64), actual: (i64, i64)) -> i64 {
    let lost_it = (full.0 - actual.0).max(0);
    let lost_rt = (full.1 - actual.1).max(0);
    let v = lost_it * 1_021 / 10_000 + lost_rt / 10;
    v / 100 * 100
}

/// 税法上の扶養判定
pub fn judge_tax(year: u32, age: u32, total_income: i64) -> DependentJudge {
    let line = tax_line(year);
    let value = total_income.max(0);

    if age < 16 {
        return DependentJudge {
            status: Status::Na,
            label: "対象外".into(),
            value,
            line,
            message: "16歳未満はもともと親の扶養控除がない年齢なので、親の税金への影響はありません。".into(),
            parent_tax_increase: 0,
        };
    }

    let specific = (19..=22).contains(&age);
    let full = if specific { (63 * MAN, 45 * MAN) } else { (38 * MAN, 33 * MAN) };
    let actual = parent_deduction(year, age, value);
    let increase = parent_increase(full, actual);

    if value <= line {
        let room = line - value;
        let (status, label) = if room <= 10 * MAN {
            (Status::Caution, "ギリギリ")
        } else {
            (Status::Safe, "セーフ")
        };
        return DependentJudge {
            status,
            label: label.into(),
            value,
            line,
            message: format!(
                "所得{}は{}年のライン{}以内。あと{}増えるとアウト。",
                fmt_yen(value),
                year,
                fmt_yen(line),
                fmt_yen(room + 1)
            ),
            parent_tax_increase: 0,
        };
    }

    if specific && value <= SPECIAL_RELATIVE_UPPER {
        return DependentJudge {
            status: Status::Caution,
            label: "一部アウト".into(),
            value,
            line,
            message: format!(
                "所得{}で{}を超えたため、親の控除が「特定扶養63万円」→「特定親族特別控除{}」に減ります。親の年末調整・確定申告の書類変更が必要で、親の税金が年 約{}増えます。",
                fmt_yen(value),
                fmt_yen(line),
                fmt_yen(actual.0),
                fmt_yen(increase)
            ),
            parent_tax_increase: increase,
        };
    }

    DependentJudge {
        status: Status::Out,
        label: "アウト".into(),
        value,
        line,
        message: format!(
            "所得{}がライン{}を超えて親の扶養から外れます。親が扶養控除を使っていたら年末調整・確定申告のやり直しが必要で、親側に約{}の追加納税が発生します。",
            fmt_yen(value),
            fmt_yen(if specific { SPECIAL_RELATIVE_UPPER } else { line }),
            fmt_yen(increase)
        ),
        parent_tax_increase: increase,
    }
}

/// 健康保険の扶養の収入ライン
pub fn insurance_line(age: u32) -> i64 {
    if (19..=22).contains(&age) {
        150 * MAN
    } else {
        130 * MAN
    }
}

/// 健康保険（社会保険）の扶養判定
///
/// * `annual_estimate` - 年間収入見込み（振込−経費＋給与）
/// * `pace_estimate` - 始めた月からの月平均ペースで年換算した見込み（途中開始の年のみ）
/// * `own_shaho` - 本人が勤務先の社会保険に加入している
pub fn judge_insurance(age: u32, annual_estimate: i64, pace_estimate: i64, own_shaho: bool) -> DependentJudge {
    let line = insurance_line(age);
    let value = annual_estimate.max(0);

    if own_shaho {
        return DependentJudge {
            status: Status::Na,
            label: "自分で加入".into(),
            value,
            line,
            message: "勤務先の健康保険に自分で入っているので、親の健康保険の扶養は関係ありません。".into(),
            parent_tax_increase: 0,
        };
    }

    let line_txt = fmt_yen(line);
    if value >= line {
        return DependentJudge {
            status: Status::Out,
            label: "アウト".into(),
            value,
            line,
            message: format!(
                "年間収入の見込み{}が{}以上。親の保険証から抜けて国保へ切り替えが必要。抜けるべき日以降に親の保険で病院にかかった分は、医療費（7割分）の返還を求められるリスクがあります。",
                fmt_yen(value),
                line_txt
            ),
            parent_tax_increase: 0,
        };
    }
    if pace_estimate >= line {
        return DependentJudge {
            status: Status::Caution,
            label: "グレー".into(),
            value,
            line,
            message: format!(
                "年合計{}はライン未満でも、月平均ペースだと年換算{}。健康保険は「今後1年の見込み」で判断するため、始めた月から扶養外と見なされる可能性があります（加入先の健保組合に確認）。",
                fmt_yen(value),
                fmt_yen(pace_estimate)
            ),
            parent_tax_increase: 0,
        };
    }
    if value >= line * 85 / 100 {
        return DependentJudge {
            status: Status::Caution,
            label: "ギリギリ".into(),
            value,
            line,
            message: format!(
                "年間収入の見込み{}。あと{}で{}の壁を超えて扶養から外れます。",
                fmt_yen(value),
                fmt_yen(line - value),
                line_txt
            ),
            parent_tax_increase: 0,
        };
    }
    DependentJudge {
        status: Status::Safe,
        label: "セーフ".into(),
        value,
        line,
        message: format!("年間収入の見込み{}は{}未満。親の保険証のままでOK。", fmt_yen(value), line_txt),
        parent_tax_increase: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tax_lines_by_year() {
        assert_eq!(judge_tax(2025, 25, 58 * MAN).status, Status::Caution); // ギリギリだがセーフ側
        assert_eq!(judge_tax(2025, 25, 40 * MAN).status, Status::Safe);
        assert_eq!(judge_tax(2025, 25, 58 * MAN + 1).status, Status::Out);
        assert_eq!(judge_tax(2026, 25, 60 * MAN).status, Status::Caution);
        assert_eq!(judge_tax(2026, 25, 62 * MAN + 1).status, Status::Out);
    }

    #[test]
    fn specific_relative_steps() {
        // 20歳・所得100万 → 特定親族特別控除41万
        assert_eq!(parent_deduction(2026, 20, 100 * MAN), (41 * MAN, 41 * MAN));
        let j = judge_tax(2026, 20, 100 * MAN);
        assert_eq!(j.status, Status::Caution);
        // 親の増税: (63-41)万×10.21% + (45-41)万×10% = 22,462 + 4,000 = 26,462 → 26,400
        assert_eq!(j.parent_tax_increase, 26_400);
        // 123万超で完全アウト: 63万×10.21% + 45万×10% = 64,323 + 45,000 = 109,323 → 109,300
        let j = judge_tax(2026, 20, 124 * MAN);
        assert_eq!(j.status, Status::Out);
        assert_eq!(j.parent_tax_increase, 109_300);
    }

    #[test]
    fn general_dependent_out() {
        // 一般扶養: 38万×10.21% + 33万×10% = 38,798 + 33,000 = 71,798 → 71,700
        let j = judge_tax(2026, 25, 100 * MAN);
        assert_eq!(j.status, Status::Out);
        assert_eq!(j.parent_tax_increase, 71_700);
    }

    #[test]
    fn under16_is_na() {
        assert_eq!(judge_tax(2026, 15, 300 * MAN).status, Status::Na);
    }

    #[test]
    fn insurance_lines() {
        assert_eq!(judge_insurance(20, 149 * MAN, 0, false).status, Status::Caution);
        assert_eq!(judge_insurance(20, 150 * MAN, 0, false).status, Status::Out);
        assert_eq!(judge_insurance(25, 130 * MAN, 0, false).status, Status::Out);
        assert_eq!(judge_insurance(25, 50 * MAN, 0, false).status, Status::Safe);
        assert_eq!(judge_insurance(25, 50 * MAN, 200 * MAN, false).status, Status::Caution);
        assert_eq!(judge_insurance(25, 500 * MAN, 0, true).status, Status::Na);
    }
}
