//! 診断オーケストレーター（2025年分・2026年分の統合計算とアクションリスト生成）

use crate::dependent;
use crate::income_tax;
use crate::model::{Action, DiagnoseInput, DiagnoseOutput, LateFiling, Status, YearResult};
use crate::nhi;
use crate::pension;
use crate::prefectures;
use crate::resident_tax;
use crate::util::{ceil_div, floor_to, fmt_yen, MAN};
use crate::ENGINE_VERSION;

/// 診断メイン関数
pub fn diagnose(input: &DiagnoseInput) -> DiagnoseOutput {
    let pref = prefectures::get(input.prefecture);
    let age_2026 = input.age.clamp(12, 80);
    let age_2025 = age_2026.saturating_sub(1);

    // 2025年分・2026年分を順に計算
    let y2025 = calc_year(input, 2025, age_2025, None);
    let y2026 = calc_year(input, 2026, age_2026, Some(&y2025));

    // 2025年分の期限後申告（無申告加算税・延滞税）チェック
    let late_filing = calc_late_filing(input, &y2025);

    // 絶対に使っちゃダメなお金（2026年分の税金・国保・年金 ＋ 2025年分の未納・住民税・国保の目安）
    // すでに2025年分を申告済みなら2025年の所得税は除外、未申告なら加算税・延滞税も含める
    let unpaid_2025_it = if input.filed2025 {
        0
    } else {
        y2025.income_tax
            + late_filing.as_ref().map(|l| l.penalty + l.delinquency).unwrap_or(0)
    };
    let lock_total = y2026.total + unpaid_2025_it + y2025.resident_tax;

    // 手元に残る安全なお金（2026年の手取りベース：振込 − 経費 − 2026年分の税・保険料）
    let net_2026 = (y2026.gross - y2026.expense).max(0);
    let free_money = (net_2026 - y2026.total).max(0);

    // 振込のたびに別口座へよけておくべき割合（%）
    let set_aside_percent = if y2026.gross > 0 {
        ceil_div(y2026.total * 100, y2026.gross).clamp(5, 60)
    } else if y2025.gross > 0 {
        ceil_div(y2025.total * 100, y2025.gross).clamp(5, 60)
    } else {
        0
    };

    // 総合ステータス
    let mut overall = Status::Safe;
    for y in [&y2025, &y2026] {
        overall = overall.max(y.tax_dependent.status).max(y.insurance_dependent.status);
    }
    if late_filing.is_some() {
        overall = Status::Out;
    } else if !input.filed2025 && y2025.filing_required {
        overall = Status::Out;
    }

    let headline = build_headline(overall, &y2025, &y2026, &late_filing);
    let actions = build_actions(input, &y2025, &y2026, &late_filing);
    let notes = build_notes(input, pref.name, pref.city);

    DiagnoseOutput {
        engine_version: ENGINE_VERSION.to_string(),
        prefecture_name: pref.name.to_string(),
        prefecture_city: pref.city.to_string(),
        years: vec![y2025, y2026],
        late_filing,
        lock_total,
        free_money,
        set_aside_percent,
        overall,
        headline,
        actions,
        notes,
    }
}

fn calc_year(
    input: &DiagnoseInput,
    year: u32,
    age: u32,
    prev: Option<&YearResult>,
) -> YearResult {
    let (gross_raw, exp_raw) = if year == 2025 {
        (input.income2025, input.expense2025)
    } else {
        (input.income2026, input.expense2026)
    };
    let gross = gross_raw.max(0);
    let expense = exp_raw.max(0).min(gross);
    let profit_before_blue = (gross - expense).max(0);

    // 青色申告特別控除（65万円、ただし所得金額が上限）
    let blue_deduction = if input.blue_return {
        profit_before_blue.min(65 * MAN)
    } else {
        0
    };
    let business_income = (profit_before_blue - blue_deduction).max(0);

    // 給与所得
    let salary_raw = if input.has_salary {
        input.salary_annual.max(0)
    } else {
        0
    };
    let salary_inc_it = income_tax::salary_income(year, salary_raw);
    let salary_inc_rt = income_tax::salary_income_for_resident(year, salary_raw);

    let total_income_it = business_income + salary_inc_it;
    let total_income_rt = business_income + salary_inc_rt;

    // 親の扶養判定（税法・社会保険）
    let tax_dep = dependent::judge_tax(year, age, total_income_it);

    // 健康保険の年間収入見込み（配信は経費差引後の手残り＋給与額面で判定するのが実務上一般的）
    let annual_est = profit_before_blue + salary_raw;
    let pace_est = if year == 2025 && (1..=12).contains(&input.start_month2025) {
        let months = (13 - input.start_month2025).max(1) as i64;
        profit_before_blue * 12 / months + salary_raw
    } else {
        annual_est
    };
    let ins_dep = dependent::judge_insurance(age, annual_est, pace_est, input.salary_has_shaho);

    // 国保（健保の扶養から外れる場合のみ本人に発生、または一人暮らしで国保加入の場合）
    let needs_own_nhi = !input.salary_has_shaho
        && (ins_dep.status == Status::Out || input.lives_alone && annual_est >= 130 * MAN);
    let nhi_calc = if needs_own_nhi {
        nhi::calc(
            input.prefecture,
            total_income_rt,
            age,
            input.lives_alone,
            input.has_salary,
        )
    } else {
        nhi::NhiCalc {
            premium: 0,
            reduction: 0,
        }
    };

    // 国民年金（20歳以上・会社の社保なし）
    // ただし「収入0円」または「学生・低所得で免除/猶予ライン以下かつ配信収入が少ない」場合は
    // 手取りが年金だけで0円になって絶望しないよう、実態に合わせて以下とする：
    // - 配信も給与も0円の年は 0円
    // - 学生世代(19〜22歳)で前年・当年の合計所得が学生納付特例ライン(128万)以下、または
    //   一般世代で全額免除ライン(67万)以下の場合は、申請により0円（猶予・免除）にできるため 0円計上し、アクションで申請案内を出す
    let prev_inc = prev.map(|p| p.total_income).unwrap_or(total_income_it);
    let judge_inc = total_income_it.max(prev_inc);
    let can_defer_or_exempt = if (19..=22).contains(&age) {
        judge_inc <= pension::STUDENT_DEFERMENT_LINE
    } else {
        judge_inc <= pension::FULL_EXEMPTION_LINE
    };
    let pension_amt = if gross == 0 && salary_raw == 0 || can_defer_or_exempt {
        0
    } else {
        pension::annual(year, age, input.salary_has_shaho)
    };

    // 社会保険料控除（その年に払う国保＋年金の目安）
    let social_deduction = nhi_calc.premium + pension_amt;

    // 所得税（給与がある場合は「給与＋配信」の税額 −「給与のみ」の税額＝配信で増える分）
    // 給与側ですでに使われる社会保険料控除・基礎控除と重複しないよう、給与のみの税額は社保控除0で比較し、配信利益を超えないようクリップ
    let it_full = income_tax::calc(year, total_income_it, social_deduction);
    let it_sal_only = if input.has_salary {
        income_tax::calc(year, salary_inc_it, 0).tax
    } else {
        0
    };
    let income_tax_amt = (it_full.tax - it_sal_only).max(0).min(profit_before_blue);

    // 住民税（翌年6月〜：賦課期日の年齢は age + 1）
    let rt_full = resident_tax::calc(total_income_rt, social_deduction, age + 1);
    let rt_sal_only = if input.has_salary {
        resident_tax::calc(salary_inc_rt, 0, age + 1)
    } else {
        0
    };
    let resident_tax_amt = (rt_full - rt_sal_only).max(0).min(profit_before_blue);

    // 確定申告が必要か
    // - 給与あり（年末調整済み想定）: 配信の所得が 20万円超で確定申告が必要
    // - 給与なし（配信のみ）: 合計所得が基礎控除超（または青色65万控除を使うため申告必要）
    let filing_required = if input.has_salary && salary_raw > 0 {
        business_income > 20 * MAN || (input.blue_return && profit_before_blue > 20 * MAN)
    } else {
        total_income_it > it_full.basic_deduction
            || (input.blue_return && profit_before_blue > it_full.basic_deduction)
            || (year == 2025 && profit_before_blue > 48 * MAN)
    };

    let resident_filing_required = !filing_required && total_income_rt > resident_tax::EXEMPT_LINE;

    let total = income_tax_amt + resident_tax_amt + nhi_calc.premium + pension_amt;

    YearResult {
        year,
        age,
        gross,
        expense,
        blue_deduction,
        business_income,
        salary_income: salary_inc_it,
        total_income: total_income_it,
        basic_deduction: it_full.basic_deduction,
        social_deduction,
        taxable_income: it_full.taxable_income,
        income_tax: income_tax_amt,
        resident_tax: resident_tax_amt,
        resident_tax_label: resident_tax::fiscal_label(year),
        nhi: nhi_calc.premium,
        nhi_label: nhi::fiscal_label(year),
        nhi_reduction: nhi_calc.reduction,
        pension: pension_amt,
        total,
        tax_dependent: tax_dep,
        insurance_dependent: ins_dep,
        filing_required,
        resident_filing_required,
    }
}

/// 2025年分の期限後申告（無申告加算税・延滞税）の判定と計算
///
/// - 2025年の配信所得（青色控除前）が48万円（または58万円）を超えていて未申告なら警告
/// - 自主的な期限後申告の無申告加算税は 5%（100円未満切り捨て、5,000円未満は不徴収）
/// - 延滞税（令和8年分）: 納期限（2026/3/16）の翌日から2か月以内は年2.8%、2か月超は年9.1%
fn calc_late_filing(input: &DiagnoseInput, y2025: &YearResult) -> Option<LateFiling> {
    if input.filed2025 {
        return None;
    }
    let profit_2025 = (y2025.gross - y2025.expense).max(0);
    let threshold = if input.has_salary && input.salary_annual > 0 {
        20 * MAN
    } else {
        48 * MAN
    };
    if profit_2025 <= threshold && !y2025.filing_required {
        return None;
    }

    let tax = y2025.income_tax;
    let days = input.days_since_2025_deadline.max(0);

    // 無申告加算税（税務署から指摘される前に自主申告すれば 5%）
    let raw_penalty = floor_to(tax * 5 / 100, 100);
    let penalty = if raw_penalty < 5_000 { 0 } else { raw_penalty };

    // 延滞税（本税1万円未満切り捨て、1,000円未満の場合は0、算出額1,000円未満も0）
    let tax_base = floor_to(tax, 10_000);
    let delinquency = if tax_base < 10_000 || days == 0 {
        0
    } else {
        let d1 = days.min(61);
        let d2 = (days - 61).max(0);
        let interest = tax_base * 28 * d1 / 365_000 + tax_base * 91 * d2 / 365_000;
        let rounded = floor_to(interest, 100);
        if rounded < 1_000 { 0 } else { rounded }
    };

    let msg = if tax > 0 {
        format!(
            "2025年（昨年）分の所得が{}あり、確定申告期限（2026年3月16日）を過ぎています！税務署から連絡が来る前に自主的に「期限後申告」すれば、無申告加算税は5%（約{}）に軽減されます（指摘後だと15〜20%）。",
            fmt_yen(y2025.business_income),
            fmt_yen(penalty)
        )
    } else {
        format!(
            "2025年（昨年）分の配信所得が{}（48万円超）あります。基礎控除の引き上げで所得税が0円でも、住民税や親の扶養判定のために申告が必要です。早めに期限後申告（または市区町村への住民税申告）を出しましょう。",
            fmt_yen(profit_2025)
        )
    };

    Some(LateFiling {
        tax,
        penalty,
        delinquency,
        message: msg,
    })
}

fn build_headline(
    overall: Status,
    y2025: &YearResult,
    y2026: &YearResult,
    late: &Option<LateFiling>,
) -> String {
    if late.is_some() {
        return "🚨 昨年（2025年）分の無申告アラートあり！今すぐ期限後申告＆親への連絡が必要です".into();
    }
    match overall {
        Status::Out => {
            if y2026.insurance_dependent.status == Status::Out {
                "🔴 親の「税金の扶養」も「健康保険証」もアウト！保険証の切り替えと納税資金の確保を".into()
            } else {
                "🔴 親の「税金の扶養」から外れます！年末調整の伝え直しと納税資金の確保を".into()
            }
        }
        Status::Caution => {
            "🟡 扶養の壁ギリギリ（または特定親族特別控除ゾーン）！年末までの振込ペースに注意".into()
        }
        Status::Safe | Status::Na => {
            if y2025.gross == 0 && y2026.gross == 0 {
                "🎙️ 上のマイクで喋るかスライダーを動かすと、税金・国保・親の扶養を一撃診断します".into()
            } else {
                "🟢 親の扶養も保険証もセーフ！領収書・スクショだけ残しておけば安心です".into()
            }
        }
    }
}

fn build_actions(
    input: &DiagnoseInput,
    y2025: &YearResult,
    y2026: &YearResult,
    late: &Option<LateFiling>,
) -> Vec<Action> {
    let mut list = Vec::new();

    // 1. 2025年分の期限後申告
    if let Some(lf) = late {
        list.push(Action {
            priority: Status::Out,
            icon: "🚨".into(),
            title: "昨年（2025年）分の「期限後申告」を今すぐ出す".into(),
            detail: format!(
                "{} マイナンバーカード＋スマホ（e-Tax）で自宅から提出できます。",
                lf.message
            ),
            when: "今すぐ（1日でも早く）".into(),
        });
    }

    // 2. 健康保険の扶養アウト（2025 or 2026）
    if y2025.insurance_dependent.status == Status::Out
        || y2026.insurance_dependent.status == Status::Out
    {
        list.push(Action {
            priority: Status::Out,
            icon: "🪪".into(),
            title: "親に健康保険証の件を話し、市区町村役場で「国民健康保険」に切り替える".into(),
            detail: "親の勤務先に「被扶養者異動届」を出して資格喪失証明書をもらい、14日以内に役所の国保窓口（またはマイナポータル）で加入手続きをします。放置すると後から病院代7割分の返還請求が親に届きます。".into(),
            when: "収入見込みが壁を超えた時点から14日以内".into(),
        });
    } else if y2025.insurance_dependent.status == Status::Caution
        || y2026.insurance_dependent.status == Status::Caution
    {
        list.push(Action {
            priority: Status::Caution,
            icon: "🪪".into(),
            title: "親の健康保険組合の「扶養ルール（自営業の経費の扱い）」を確認する".into(),
            detail: "健康保険組合によっては「経費を引く前の売上（振込額）」で130万/150万を判定するところがあります。親の保険証に記載の健保組合名をググるか親に確認しましょう。".into(),
            when: "今月中".into(),
        });
    }

    // 3. 税金の扶養アウト・特定親族特別控除
    if y2025.tax_dependent.status == Status::Out || y2026.tax_dependent.status == Status::Out {
        let inc_max = y2025
            .tax_dependent
            .parent_tax_increase
            .max(y2026.tax_dependent.parent_tax_increase);
        list.push(Action {
            priority: Status::Out,
            icon: "👪".into(),
            title: "親に「今年の年末調整で私を扶養から外して」とLINEする".into(),
            detail: format!(
                "この診断カードをスクショして親に送ればOK！親の税金が年 約{}増えるので、その分を配信の稼ぎから親に渡すと揉めません。",
                fmt_yen(inc_max)
            ),
            when: "親の会社の年末調整（11〜12月）の前まで".into(),
        });
    } else if y2025.tax_dependent.status == Status::Caution
        || y2026.tax_dependent.status == Status::Caution
    {
        list.push(Action {
            priority: Status::Caution,
            icon: "👪".into(),
            title: "親に「今年の所得の見込み額」を共有しておく（特定親族特別控除）".into(),
            detail: "19〜22歳は所得62万円（2025年は58万円）を超えても123万円以下なら「特定親族特別控除」で親の控除が段階的に残りますが、親が年末調整の紙にあなたの所得見込みを正確に書く必要があります。".into(),
            when: "11月〜12月（親の年末調整時）".into(),
        });
    }

    // 4. 青色申告の提案（まだ白色で、所得がそれなりにある場合）
    let profit_2026 = (y2026.gross - y2026.expense).max(0);
    if !input.blue_return && profit_2026 > 65 * MAN {
        list.push(Action {
            priority: Status::Caution,
            icon: "📘".into(),
            title: "「開業届」＋「青色申告承認申請書」を出して65万円控除を使う".into(),
            detail: "スマホ会計アプリ（freee・マネーフォワード・弥生など）から無料で書類を作ってマイナポータル送信できます。最大65万円を所得から引けるので、税金・国保が一気に安くなり親の扶養にも残りやすくなります。".into(),
            when: "青色にしたい年の3月15日まで（新規開業は開業から2か月以内）".into(),
        });
    }

    // 5. 国民年金の免除・学生納付特例
    if y2026.pension > 0 {
        let hint = pension::exemption_hint(y2025.total_income, y2026.age);
        list.push(Action {
            priority: Status::Safe,
            icon: "年金".into(),
            title: "20歳以上の「国民年金」の手続き・学生納付特例をチェック".into(),
            detail: hint,
            when: "20歳の誕生月以降／毎年4月".into(),
        });
    }

    // 6. 住民税の申告（所得税は0円だけど住民税の申告が必要なケース）
    if y2026.resident_filing_required {
        list.push(Action {
            priority: Status::Caution,
            icon: "🏛️".into(),
            title: "所得税は0円でも「住民税の申告」を忘れずに".into(),
            detail: "2026年は基礎控除104万円のおかげで所得税が0円でも、所得が45万円を超えているため住民税の申告が必要です（確定申告を出せば住民税申告も兼ねられます）。".into(),
            when: "2027年2月16日〜3月15日".into(),
        });
    }

    // 7. 経費の領収書・スクショ保存（全員共通）
    list.push(Action {
        priority: Status::Safe,
        icon: "🧾".into(),
        title: "配信で使ったお金の「スクショ・レシート」をスマホのアルバムに保存".into(),
        detail: "リングライト・マイク・スマホスタンド・配信用の衣装やコスメ・BGMやスタンプ代・コラボ配信の交通費・自宅の家賃や通信費の一部（按分）は経費になります。経費が増えるほど所得が減って税金も扶養判定も有利に！".into(),
        when: "今日から毎日".into(),
    });

    list
}

fn build_notes(input: &DiagnoseInput, pref_name: &str, pref_city: &str) -> Vec<String> {
    let mut notes = vec![
        format!(
            "国保料は「{}（{}）」の40歳未満・介護分なし代表料率（令和8年度・子ども子育て支援金分含む）による概算です。実際の金額はお住まいの市区町村により異なります。",
            pref_name, pref_city
        ),
        "2025年分は令和7年度税制（基礎控除58万〜最大95万）、2026年分は令和8年度税制改正（基礎控除 本則62万・特例最大104万、扶養の所得要件62万、給与所得控除最低74万）を適用しています。".into(),
        "親の増税額は「親の所得税率10%＋住民税率10%」の一般的な世帯を想定した目安です（親の年収が高い場合はさらに増えることがあります）。".into(),
    ];
    if !input.lives_alone {
        notes.push("親と同居（親が世帯主）として国保の低所得軽減は「なし」で保守的に計算しています（一人暮らしで自分が世帯主の場合は2〜7割軽減が効くことがあります）。".into());
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_example_scenario() {
        // 「20歳で東京に住んでて、去年10月から始めて50万、今年は250万振り込まれて、経費は30万くらい」
        let input = DiagnoseInput {
            age: 20,
            prefecture: 12, // 東京
            income2025: 50 * MAN,
            expense2025: 0,
            income2026: 250 * MAN,
            expense2026: 30 * MAN,
            start_month2025: 10,
            days_since_2025_deadline: 245,
            ..Default::default()
        };
        let out = diagnose(&input);
        assert_eq!(out.years.len(), 2);

        let y25 = &out.years[0];
        let y26 = &out.years[1];

        // 2025年: 所得50万 → 19歳なので税の扶養(58万以下)はセーフ側(ギリギリCaution)、ただし48万超未申告なのでLateFilingあり
        assert_eq!(y25.business_income, 50 * MAN);
        assert_eq!(y25.income_tax, 0); // 基礎控除95万なので所得税は0
        assert!(out.late_filing.is_some());
        // 10月開始で3か月50万 → 年換算200万ペースなので健保はグレー(Caution)
        assert_eq!(y25.insurance_dependent.status, Status::Caution);

        // 2026年: 250万 - 30万 = 所得220万 → 税の扶養(123万超)も健保(150万以上)もアウト
        assert_eq!(y26.business_income, 220 * MAN);
        assert_eq!(y26.tax_dependent.status, Status::Out);
        assert_eq!(y26.insurance_dependent.status, Status::Out);
        assert!(y26.income_tax > 0);
        assert!(y26.resident_tax > 0);
        assert!(y26.nhi > 0);
        assert_eq!(y26.pension, 215_040);
        assert!(out.lock_total > 40 * MAN);
    }
}
