use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const KEY: &str = "tiktok_liver_tax_ledger_v1";
pub const INCOME: &[&str] = &["振込売上（TikTok・投げ銭）", "イベント・案件報酬"];
pub const EXPENSE: &[&str] = &[
    "消耗品費（機材・ライト・マイク）",
    "衣装・美容費（配信用コスメ・服）",
    "通信費（スマホ・回線）",
    "地代家賃（配信部屋の家賃按分）",
    "旅費交通費（コラボ・イベント移動）",
    "接待交際費・雑費（企画・BGM等）",
];
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: String,
    pub amount: i64,
    pub memo: String,
}
pub struct Summary {
    pub income: i64,
    pub expense: i64,
    pub monthly: [(i64, i64); 12],
    pub categories: BTreeMap<String, i64>,
}
pub fn summarize(entries: &[Entry], year: i32) -> Summary {
    let mut s = Summary {
        income: 0,
        expense: 0,
        monthly: [(0, 0); 12],
        categories: BTreeMap::new(),
    };
    for e in entries
        .iter()
        .filter(|e| e.date.starts_with(&format!("{year}-")))
    {
        let m = e
            .date
            .get(5..7)
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| (1..=12).contains(v))
            .unwrap_or(1)
            - 1;
        if e.kind == "income" {
            s.income += e.amount;
            s.monthly[m].0 += e.amount;
        } else {
            s.expense += e.amount;
            s.monthly[m].1 += e.amount;
            *s.categories.entry(e.category.clone()).or_default() += e.amount;
        }
    }
    s
}
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub fn csv(entries: &[Entry]) -> String {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let mut s = String::from("\u{feff}日付,年度,区分,勘定科目,金額(円),摘要・メモ\n");
    for e in sorted {
        s += &format!(
            "{},{},{},{},{},{}\n",
            quote(&e.date),
            quote(&format!("{}年", e.date.get(..4).unwrap_or(""))),
            quote(if e.kind == "income" {
                "売上(収入)"
            } else {
                "経費(支出)"
            }),
            quote(&e.category),
            e.amount,
            quote(&e.memo)
        );
    }
    s
}
pub fn valid_date(y: i32, m: u32, d: u32) -> bool {
    (2025..=2026).contains(&y) && (1..=12).contains(&m) && d > 0 && d <= days(y, m)
}
pub fn days(y: i32, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_data_and_csv() {
        let mut e: Vec<Entry> = serde_json::from_str(include_str!("sample-ledger.json")).unwrap();
        assert_eq!(summarize(&e, 2025).income, 500000);
        e[0].memo = "引用\"と,改行\n".into();
        assert!(csv(&e).contains("\"引用\"\"と,改行\n\""));
        assert!(!valid_date(2026, 2, 29));
        assert!(valid_date(2026, 2, 28));
    }
}
