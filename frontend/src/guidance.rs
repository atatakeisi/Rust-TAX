use rust_tax_engine::model::{DiagnoseInput, DiagnoseOutput, HealthCoverage, Status};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STORAGE_KEY: &str = "rust_tax_guidance_checked_v1";
pub type Checked = BTreeMap<Topic, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    PriorReturn,
    Health,
    Resident,
    IncomeTax,
    Blue,
}

impl Topic {
    pub const ALL: [Self; 5] = [
        Self::PriorReturn,
        Self::Health,
        Self::Resident,
        Self::IncomeTax,
        Self::Blue,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::PriorReturn => "prior-return",
            Self::Health => "health",
            Self::Resident => "resident",
            Self::IncomeTax => "income-tax",
            Self::Blue => "blue",
        }
    }

    pub fn category(self) -> (&'static str, &'static str) {
        match self {
            Self::PriorReturn | Self::IncomeTax => ("tax", "所得税"),
            Self::Health => ("health", "健康保険"),
            Self::Resident => ("resident", "住民税"),
            Self::Blue => ("blue", "青色申告"),
        }
    }
}

pub struct Step {
    pub topic: Topic,
    pub title: &'static str,
    pub preview: &'static str,
    pub detail: &'static str,
    pub priority: u8,
    pub review_scope: &'static str,
}

pub fn has_amounts(input: &DiagnoseInput) -> bool {
    input.income2025 > 0 || input.income2026 > 0 || (input.has_salary && input.salary_annual > 0)
}

pub fn steps(input: &DiagnoseInput, out: &DiagnoseOutput) -> [Step; 5] {
    let prior = &out.years[0];
    let current = &out.years[1];
    let prior_review =
        prior.gross > 0 && !input.filed2025 && (prior.filing_required || out.late_filing.is_some());
    let (prior_preview, prior_detail, prior_priority) = if input.filed2025 {
        (
            "2025年分は申告済みと入力されています。",
            "控えを保管し、配信収入に漏れがないか確認しましょう。漏れに気づいたときは、控えや振込・経費の記録を用意して税務署に相談できます。",
            0,
        )
    } else if prior.gross > 0 {
        (
            "2025年分の振込と経費をまとめて、申告の要否を確認しましょう。",
            "申告が必要でまだ提出していない場合は、早めに税務署へ。状況に応じて無申告加算税や延滞税がかかることがあります。すでに申告した場合は、申告状況を更新できます。",
            if prior_review { 6 } else { 2 },
        )
    } else {
        (
            "2025年にも配信収入があった場合は、その年の金額も入力しましょう。",
            "2025年分の入力欄から振込・経費・申告状況を確認できます。",
            0,
        )
    };

    let coverage = match input.health_coverage_2026 {
        HealthCoverage::Unknown if input.salary_has_shaho => HealthCoverage::Employer,
        coverage => coverage,
    };
    let health_risk = matches!(
        current.insurance_dependent.status,
        Status::Caution | Status::Out
    ) || (prior.gross > 0
        && matches!(
            prior.insurance_dependent.status,
            Status::Caution | Status::Out
        ));
    let (health_title, health_preview, health_detail, health_priority) = match coverage {
        HealthCoverage::Unknown => (
            "いまの健康保険を確認する",
            "親の扶養、勤務先の保険、国保のどれに入っているかを確認しましょう。",
            "親の扶養なら、今後の収入見込みを親の勤務先や保険者に伝え、資格と適用日を確認してください。税金の扶養とは別の手続きです。",
            if health_risk { 5 } else { 3 },
        ),
        HealthCoverage::Dependent => (
            "親の健康保険の扶養を確認する",
            "配信と給与の収入見込みを伝え、扶養を続けられるか保険者に確認しましょう。",
            "資格を失った日が確認できたら、以前の資格で受診せず、次の保険を確認します。勤務先の健康保険などに入らない場合は市区町村で国保の手続きへ。届出が遅れると資格取得日まで遡った保険料が発生することがあり、旧資格で受診した分の返還を求められる場合もあります。",
            if health_risk { 5 } else { 2 },
        ),
        HealthCoverage::Employer => (
            "勤務先の健康保険を確認する",
            "資格情報で加入日を確認し、以前の保険があれば切替手続きを確認しましょう。",
            "以前に国保へ加入していた場合は、市区町村に脱退の届出を確認してください。",
            1,
        ),
        HealthCoverage::National => (
            "国保の納付書を確認する",
            "市区町村から届く通知で、実際の保険料と納期限を確認しましょう。",
            "届出日と資格取得日が違う場合は、通知に遡及分が含まれることがあります。金額や納期限が分からなければ、市区町村の国保窓口へ。",
            2,
        ),
    };

    let (resident_preview, resident_priority) = if prior.resident_tax > 0 {
        (
            "2025年分の所得に対する2026年度の通知書や給与明細を確認しましょう。",
            4,
        )
    } else if current.gross > 0 {
        (
            "2026年分の所得に対する2027年度の通知の受け取り方を確認しましょう。",
            2,
        )
    } else {
        ("住民税の通知が届いたら、金額と納期限を確認しましょう。", 2)
    };
    [
        Step {
            topic: Topic::PriorReturn,
            title: "2025年分の確定申告を確認する",
            preview: prior_preview,
            detail: prior_detail,
            priority: prior_priority,
            review_scope: match (input.filed2025, prior_review, prior.gross > 0) {
                (true, _, _) => "filed",
                (_, true, _) => "needs_attention",
                (_, _, true) => "has_income",
                _ => "no_income_entered",
            },
        },
        Step {
            topic: Topic::Health,
            title: health_title,
            preview: health_preview,
            detail: health_detail,
            priority: health_priority,
            review_scope: match (coverage, health_risk) {
                (HealthCoverage::Unknown, true) => "unknown_risk",
                (HealthCoverage::Unknown, false) => "unknown",
                (HealthCoverage::Dependent, true) => "dependent_risk",
                (HealthCoverage::Dependent, false) => "dependent",
                (HealthCoverage::Employer, _) => "employer",
                (HealthCoverage::National, _) => "national",
            },
        },
        Step {
            topic: Topic::Resident,
            title: "住民税の通知と納期限を確認する",
            preview: resident_preview,
            detail: "自分で払う場合は自治体の通知書、給与から引かれる場合は勤務先からの通知や給与明細を確認します。所得税の確定申告をしない場合も、住民税の申告が必要なことがあります。通知が見つからないときは市区町村の住民税窓口へ。",
            priority: resident_priority,
            review_scope: if prior.resident_tax > 0 { "2026_notice" } else { "next_notice" },
        },
        Step {
            topic: Topic::IncomeTax,
            title: "2026年分の確定申告に備える",
            preview: "振込明細と領収書を集め、給与があれば源泉徴収票も保管しましょう。",
            detail: "申告が必要かどうかは、配信や給与の所得、控除、給与の受け取り方などで変わります。国税庁の案内で確認し、申告や納付の期限を控えておきましょう。",
            priority: if current.filing_required { 3 } else { 1 },
            review_scope: if current.filing_required { "needs_attention" } else { "prepare" },
        },
        Step {
            topic: Topic::Blue,
            title: "青色申告の申請期限を確認する",
            preview: "青色申告を検討するなら、対象になる年と帳簿の要件を早めに確認しましょう。",
            detail: "開業届と青色申告承認申請書は別の手続きです。青色申告特別控除には、所得の区分、帳簿付け、期限内申告などの要件があります。",
            priority: u8::from(!input.blue_return_2026 && current.gross > 0),
            review_scope: if input.blue_return_2026 { "using_blue" } else { "consider_blue" },
        },
    ]
}
