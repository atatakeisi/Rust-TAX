//! 入出力のデータ構造（JS とは camelCase の JSON でやり取りする）

use serde::{Deserialize, Serialize};

/// 2026年に本人が利用している健康保険
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthCoverage {
    #[default]
    Unknown,
    Dependent,
    Employer,
    National,
}

/// 診断の入力
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiagnoseInput {
    /// 現在（2026年時点）の年齢。年齢区分ボタンの場合は代表値（17 / 20 / 25）
    pub age: u32,
    /// 都道府県インデックス（0=北海道 … 46=沖縄県, 47=わからない（全国平均））
    pub prefecture: usize,
    /// 2025年（昨年）1〜12月の振込合計（円）
    pub income2025: i64,
    /// 2025年の経費概算（円）
    pub expense2025: i64,
    /// 2026年（今年）1〜12月の振込合計（見込み含む・円）
    pub income2026: i64,
    /// 2026年の経費概算（円）
    pub expense2026: i64,
    /// 本業（バイト・会社員給与）の有無
    pub has_salary: bool,
    /// バイト代・給与の年額（額面・円）。両年共通の目安として使う
    pub salary_annual: i64,
    /// 勤務先の社会保険に本人が加入しているか。2025年と、加入先未確認の2026年で使う
    pub salary_has_shaho: bool,
    /// 2026年の実際の加入先。指定時は2026年のみ既存の加入情報より優先する
    pub health_coverage_2026: HealthCoverage,
    /// 2025年分は期限内に青色申告し、65万円控除を適用したか
    pub blue_return_2025: bool,
    /// 2026年分の青色申告承認を受け、65万円控除の要件を満たす見込みか
    pub blue_return_2026: bool,
    /// 2025年分の確定申告をすでに出したか
    pub filed2025: bool,
    /// 一人暮らし（自分が世帯主）か。false = 親と同居（親が世帯主）
    pub lives_alone: bool,
    /// 配信を始めた月。0 = 2024年以前から（2025年は通年）, 1〜12 = 2025年のその月, 13 = 2026年から
    pub start_month2025: u32,
    /// 2025年分の申告期限（2026/3/16）からの経過日数（JS 側で今日の日付から計算して渡す）
    pub days_since_2025_deadline: i64,
}

impl Default for DiagnoseInput {
    fn default() -> Self {
        Self {
            age: 20,
            prefecture: 47, // わからない（全国平均・身バレ防止デフォルト）
            income2025: 0,
            expense2025: 0,
            income2026: 0,
            expense2026: 0,
            has_salary: false,
            salary_annual: 0,
            salary_has_shaho: false,
            health_coverage_2026: HealthCoverage::Unknown,
            blue_return_2025: false,
            blue_return_2026: false,
            filed2025: false,
            lives_alone: false,
            start_month2025: 0,
            days_since_2025_deadline: 0,
        }
    }
}

/// 赤・黄・緑のステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// 対象外（判定不要）
    Na,
    /// 🟢 セーフ
    Safe,
    /// 🟡 注意・グレー
    Caution,
    /// 🔴 アウト
    Out,
}

/// 扶養の判定結果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependentJudge {
    pub status: Status,
    /// バッジ用の短いラベル（例:「セーフ」「アウト」）
    pub label: String,
    /// 判定に使った金額（所得 or 収入見込み）
    pub value: i64,
    /// 判定ライン
    pub line: i64,
    /// 説明メッセージ
    pub message: String,
    /// 親の税金が増える目安（年額・円）。税の扶養のみ
    pub parent_tax_increase: i64,
}

/// 年ごとの計算結果（その年の稼ぎにかかるお金）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YearResult {
    pub year: u32,
    /// その年の12/31時点のおおよその年齢
    pub age: u32,
    /// 振込合計
    pub gross: i64,
    /// 経費
    pub expense: i64,
    /// 青色申告特別控除の適用額
    pub blue_deduction: i64,
    /// 配信の所得（振込 − 経費 − 青色控除）
    pub business_income: i64,
    /// 給与所得（給与収入 − 給与所得控除）
    pub salary_income: i64,
    /// 合計所得金額
    pub total_income: i64,
    /// 所得税の基礎控除
    pub basic_deduction: i64,
    /// 社会保険料控除（国保＋年金の見込み）
    pub social_deduction: i64,
    /// 所得税の課税所得
    pub taxable_income: i64,
    /// 所得税＋復興特別所得税（配信で増える分）
    pub income_tax: i64,
    /// 住民税（配信で増える分）
    pub resident_tax: i64,
    /// 住民税の課税年度ラベル（例:「令和8年度（2026年6月〜）」）
    pub resident_tax_label: String,
    /// 国民健康保険料（年額）
    pub nhi: i64,
    /// 国保の年度ラベル
    pub nhi_label: String,
    /// 国保の軽減割合（0/2/5/7 割）
    pub nhi_reduction: u8,
    /// 国民年金（年額）
    pub pension: i64,
    /// 合計
    pub total: i64,
    /// 税の扶養
    pub tax_dependent: DependentJudge,
    /// 健康保険の扶養
    pub insurance_dependent: DependentJudge,
    /// 所得税の確定申告が必要か
    pub filing_required: bool,
    /// 住民税の申告（または確定申告）が必要か
    pub resident_filing_required: bool,
}

/// 2025年分の期限後申告の情報
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LateFiling {
    /// 本来の所得税
    pub tax: i64,
    /// 無申告加算税（自主的な期限後申告＝5%）
    pub penalty: i64,
    /// 延滞税の目安（今日までの分）
    pub delinquency: i64,
    pub message: String,
}

/// 今すぐやるべきアクション
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub priority: Status,
    pub icon: String,
    pub title: String,
    pub detail: String,
    /// 期限・タイミング（任意）
    pub when: String,
}

/// 診断の出力
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseOutput {
    pub engine_version: String,
    pub prefecture_name: String,
    pub prefecture_city: String,
    /// [2025年, 2026年]
    pub years: Vec<YearResult>,
    pub late_filing: Option<LateFiling>,
    /// 🔒 絶対に使っちゃダメなお金（納税・保険用に残す額）
    pub lock_total: i64,
    /// 💰 手元に残る安全なお金
    pub free_money: i64,
    /// 振込が来るたびに税金用に移すべき割合（%）
    pub set_aside_percent: i64,
    /// 総合ステータス
    pub overall: Status,
    /// 一言サマリー
    pub headline: String,
    pub actions: Vec<Action>,
    /// 注意書き・前提条件
    pub notes: Vec<String>,
}
