use regex::Regex;
use rust_tax_engine::model::DiagnoseInput;
fn re(s: &str) -> Regex {
    Regex::new(s).unwrap()
}
fn normalize(s: &str) -> String {
    let mut s: String = s
        .chars()
        .filter(|c| *c != ',' && *c != '，')
        .map(|c| {
            if ('０'..='９').contains(&c) {
                char::from_u32(c as u32 - 0xfee0).unwrap()
            } else {
                c
            }
        })
        .collect();
    for (a, b) in [
        ("二十歳", "20歳"),
        ("はたち", "20歳"),
        ("まんえん", "万円"),
        ("マンエン", "万円"),
        ("せんえん", "千円"),
    ] {
        s = s.replace(a, b);
    }
    re("[零〇一二三四五六七八九十百千]+")
        .replace_all(&s, |c: &regex::Captures| {
            let mut total = 0;
            let mut n = 0;
            for ch in c[0].chars() {
                if let Some(i) = "零一二三四五六七八九".chars().position(|x| x == ch) {
                    n = i as i64;
                } else if ch == '〇' {
                    n = 0;
                } else {
                    let unit = match ch {
                        '十' => 10,
                        '百' => 100,
                        '千' => 1000,
                        _ => 1,
                    };
                    total += n.max(1) * unit;
                    n = 0;
                }
            }
            (total + n).to_string()
        })
        .into_owned()
}
pub fn apply(raw: &str, s: &mut DiagnoseInput) {
    let text = normalize(raw);
    if let Some(c) = re(r"(\d{2})\s*(?:歳|才|さい)").captures(&text) {
        let n = c[1].parse().unwrap_or(20);
        if (12..=75).contains(&n) {
            s.age = n;
        }
    } else if re("高校生|未成年").is_match(&text) {
        s.age = 17;
    } else if text.contains("学生") {
        s.age = 20;
    }
    #[derive(serde::Deserialize)]
    struct Alias {
        id: usize,
        keys: Vec<String>,
    }
    let prefs: Vec<Alias> = serde_json::from_str(include_str!("pref-aliases.json")).unwrap();
    for p in prefs {
        if p.keys.iter().any(|k| text.contains(k)) {
            s.prefecture = p.id;
            break;
        }
    }
    if let Some(c) = re(r"(\d{1,2})\s*月から").captures(&text) {
        let m = c[1].parse().unwrap_or(0);
        if (1..=12).contains(&m) {
            s.start_month2025 = m;
        }
    }
    if text.contains("青色") {
        s.blue_return = !re("青色.*(ない|まだ)").is_match(&text);
    } else if text.contains("白色") {
        s.blue_return = false;
    }
    if re("未申告|申告してない|まだ出してない").is_match(&text) {
        s.filed2025 = false;
    } else if re("申告済|申告した|確定申告出した").is_match(&text) {
        s.filed2025 = true;
    }
    if re("一人暮らし|ひとり暮らし|単身").is_match(&text) {
        s.lives_alone = true;
    } else if re("実家|同居").is_match(&text) {
        s.lives_alone = false;
    }
    if re("社保加入|社会保険に加入").is_match(&text) {
        s.salary_has_shaho = true;
    }
    if re("配信のみ|ライバーのみ|(?:バイト|給与|本業).*(?:なし|してない)").is_match(&text)
    {
        s.has_salary = false;
        s.salary_annual = 0;
    } else if re("バイト|給与|給料|会社員|パート").is_match(&text) {
        s.has_salary = true;
        if let Some(c)=re(r"(?:バイト|給与|給料|パート)[^\d、。]{0,8}?(月|年|年間)?\s*(\d+(?:\.\d+)?)\s*(万|千円|円)?").captures(&text){let n=money(&c[2],c.get(3).map(|x|x.as_str()));s.salary_annual=n*if c.get(1).is_some_and(|x|x.as_str()=="月"){12}else{1};}
    }
    // Scan context markers in order, so multiple years and expenses in one sentence stay separate.
    let tokens = re(
        r"去年|昨年|2025年|令和7年|今年|2026年|令和8年|経費|バイト|給与|給料|配信|振込|ゼロ|なし|\d+(?:\.\d+)?\s*(?:万円|万|千円|円|歳|才|月|年)?",
    );
    let mut year = 2026;
    let mut expense = false;
    let mut salary = false;
    for m in tokens.find_iter(&text) {
        let t = m.as_str().trim();
        match t {
            "去年" | "昨年" | "2025年" | "令和7年" => {
                year = 2025;
                expense = false;
                salary = false;
            }
            "今年" | "2026年" | "令和8年" => {
                year = 2026;
                expense = false;
                salary = false;
            }
            "経費" => {
                expense = true;
                salary = false;
            }
            "バイト" | "給与" | "給料" => salary = true,
            "配信" | "振込" => salary = false,
            "ゼロ" | "なし" if !salary => assign(s, year, expense, 0),
            _ => {
                if salary || t.ends_with(['歳', '才', '月', '年']) {
                    continue;
                }
                if let Some(c) = re(r"^(\d+(?:\.\d+)?)\s*(万円|万|千円|円)?$").captures(t) {
                    let n = money(&c[1], c.get(2).map(|v| v.as_str()));
                    if text[m.end()..].starts_with("控除") {
                        continue;
                    }
                    assign(s, year, expense, n);
                }
            }
        }
    }
}
fn money(n: &str, u: Option<&str>) -> i64 {
    let n = n.parse::<f64>().unwrap_or(0.0);
    (n * match u {
        Some("万" | "万円") => 10000.0,
        Some("千円") => 1000.0,
        None if n < 2000.0 => 10000.0,
        _ => 1.0,
    })
    .round() as i64
}
fn assign(s: &mut DiagnoseInput, y: i32, e: bool, n: i64) {
    match (y, e) {
        (2025, false) => s.income2025 = n,
        (2025, true) => s.expense2025 = n,
        (_, false) => s.income2026 = n,
        (_, true) => s.expense2026 = n,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples() {
        let mut s = DiagnoseInput::default();
        apply("20歳で東京、去年10月から50万、今年250万、経費30万", &mut s);
        assert_eq!(
            (
                s.age,
                s.prefecture,
                s.income2025,
                s.income2026,
                s.expense2026
            ),
            (20, 12, 500000, 2500000, 300000)
        );
        apply(
            "大学生、バイト月6万、去年はゼロで今年140万、経費25万",
            &mut s,
        );
        assert_eq!(
            (s.salary_annual, s.income2025, s.income2026),
            (720000, 0, 1400000)
        );
        apply(
            "２４歳、去年八十万、今年四百万、経費六十万、青色申告",
            &mut s,
        );
        assert_eq!((s.age, s.income2026, s.expense2026), (24, 4000000, 600000));
    }
}
