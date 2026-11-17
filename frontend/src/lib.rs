mod blue_deadline;
mod ledger;
mod parser;
use ledger::Entry;
use rust_tax_engine::{
    diagnose::diagnose,
    model::{DiagnoseInput, Status},
};
use std::cell::RefCell;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Element, HtmlDialogElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
thread_local! {static APP:RefCell<App>=RefCell::new(App::default());}
#[derive(Default)]
struct App {
    input: DiagnoseInput,
    entries: Vec<Entry>,
    year: i32,
    storage_ok: bool,
    storage_protected: bool,
    speech: Option<JsValue>,
    listening: bool,
}
fn doc() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
fn el(id: &str) -> Element {
    doc()
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("missing element {id}"))
}
fn text(id: &str, s: impl AsRef<str>) {
    el(id).set_text_content(Some(s.as_ref()));
}
fn html(id: &str, s: &str) {
    el(id).set_inner_html(s);
}
fn hidden(id: &str, b: bool) {
    class(&el(id), "hidden", b);
}
fn class(e: &Element, name: &str, b: bool) {
    let _ = e.class_list().toggle_with_force(name, b);
}
fn val(id: &str) -> String {
    let e = el(id);
    if let Some(i) = e.dyn_ref::<HtmlInputElement>() {
        i.value()
    } else if let Some(i) = e.dyn_ref::<HtmlSelectElement>() {
        i.value()
    } else {
        e.unchecked_ref::<HtmlTextAreaElement>().value()
    }
}
fn setval(id: &str, v: impl ToString) {
    let e = el(id);
    let v = v.to_string();
    if let Some(i) = e.dyn_ref::<HtmlInputElement>() {
        i.set_value(&v)
    } else if let Some(i) = e.dyn_ref::<HtmlSelectElement>() {
        i.set_value(&v)
    } else {
        e.unchecked_ref::<HtmlTextAreaElement>().set_value(&v)
    }
}
fn num(id: &str) -> i64 {
    val(id).parse().unwrap_or(0)
}
fn render_blue_deadline() {
    use blue_deadline::{earliest_year, iso_date, japanese_date, parse_date, today_in_japan};

    let today = today_in_japan();
    let today_label = japanese_date(&today);
    text("blue-application-today", format!("{today_label}（日本時間）に提出する場合"));
    let date_input = el("blue-opening-date");
    let _ = date_input.set_attribute("max", &iso_date(&today));
    let raw_opened = val("blue-opening-date");
    let opened = parse_date(&raw_opened);
    let (result, detail) = if !raw_opened.is_empty()
        && !opened.as_ref().is_some_and(|date| date.get_time() <= today.get_time())
    {
        (
            "開業日を確認してください".to_string(),
            format!("{today_label}以前の日付を入力してください。"),
        )
    } else if let Some((year, deadline)) = earliest_year(&today, opened.as_ref()) {
        (
            format!("承認された場合、最短で{year}年分から"),
            format!("{year}年分の申請期限は{}です。", japanese_date(&deadline)),
        )
    } else {
        (
            "申請期限を税務署にご確認ください".to_string(),
            "この開業日に対応する休日情報を確認できません。".to_string(),
        )
    };
    text("blue-application-result", result);
    text("blue-application-detail", detail);
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn yen(v: i64) -> String {
    let raw = v.to_string();
    let (sign, s) = if let Some(s) = raw.strip_prefix('-') {
        ("-", s)
    } else {
        ("", raw.as_str())
    };
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            o.push(',')
        }
        o.push(c);
    }
    format!("{sign}{o}円")
}
fn man(v: i64) -> String {
    if v.abs() < 10000 {
        yen(v)
    } else {
        format!("{:.1}万円", v as f64 / 10000.0).replace(".0万", "万")
    }
}
fn status(s: Status) -> &'static str {
    match s {
        Status::Na => "na",
        Status::Safe => "safe",
        Status::Caution => "caution",
        Status::Out => "out",
    }
}
fn each(selector: &str, mut f: impl FnMut(Element)) {
    if let Ok(list) = doc().query_selector_all(selector) {
        for i in 0..list.length() {
            if let Some(n) = list.item(i) {
                f(n.unchecked_into())
            }
        }
    }
}
fn active(attr: &str, v: &str) {
    each(&format!("[{attr}]"), |e| {
        class(&e, "active", e.get_attribute(attr).as_deref() == Some(v))
    });
}
fn sync(s: &DiagnoseInput) {
    setval("pref-select", s.prefecture);
    active(
        "data-age-group",
        if s.age <= 18 {
            "u18"
        } else if s.age <= 22 {
            "student"
        } else {
            "adult"
        },
    );
    active(
        "data-has-salary",
        if s.has_salary { "true" } else { "false" },
    );
    hidden("salary-detail-box", !s.has_salary);
    hidden("blue-option-2025", !s.filed2025);
    for (id, n) in [
        ("inc-2025", s.income2025),
        ("exp-2025", s.expense2025),
        ("inc-2026", s.income2026),
        ("exp-2026", s.expense2026),
        ("salary", s.salary_annual),
    ] {
        let slider = el(&format!("slider-{id}"));
        let max = slider
            .get_attribute("max")
            .and_then(|x| x.parse::<i64>().ok())
            .unwrap_or(0);
        if n / 10000 > max {
            let _ = slider.set_attribute("max", &(n / 10000 + 100).to_string());
        }
        setval(&format!("slider-{id}"), n / 10000);
        text(&format!("val-{id}"), man(n));
    }
    setval("select-start-2025", s.start_month2025);
    for (id, checked) in [
        ("check-filed-2025", s.filed2025),
        ("check-blue-2025", s.filed2025 && s.blue_return_2025),
        ("check-blue-2026", s.blue_return_2026),
    ] {
        el(id).unchecked_ref::<HtmlInputElement>().set_checked(checked);
    }
}
fn render(a: &App) {
    let s = &a.input;
    sync(s);
    let out = diagnose(s);
    let y = &out.years[1];
    let _ = el("result-card").set_attribute("data-overall", status(out.overall));
    let _ = el("result-card").set_attribute(
        "data-biz-mode",
        if s.blue_return_2026 { "blue" } else { "white" },
    );
    text("result-headline", &out.headline);
    text(
        "result-meta",
        format!(
            "📍 {}（{}） ／ {}歳",
            out.prefecture_name, out.prefecture_city, s.age
        ),
    );
    let blue_case = (!s.blue_return_2026).then(|| {
        let mut scenario = s.clone();
        scenario.blue_return_2026 = true;
        diagnose(&scenario)
    });
    let blue = blue_case.as_ref().map_or(y, |scenario| &scenario.years[1]);
    text(
        "income-white-formula",
        format!(
            "振込{} − 経費{} ＝ {}",
            man(y.gross),
            man(y.expense),
            man(y.gross - y.expense),
        ),
    );
    text(
        "income-blue-formula",
        format!(
            "振込{} − 経費{} − 青色控除{} ＝ {}",
            man(blue.gross),
            man(blue.expense),
            man(blue.blue_deduction),
            man(blue.business_income),
        ),
    );
    let mode = if s.blue_return_2026 { "青色" } else { "白色" };
    text("hero-cost-title", format!("2026年分の負担目安（{mode}）"));
    let salary = s.salary_annual.max(0);
    hidden("income-salary-summary", !s.has_salary);
    hidden("income-tax-note", !s.has_salary || salary == 0);
    if s.has_salary {
        text(
            "income-total-formula",
            format!(
                "収入合計　振込{} ＋ 給与{} ＝ {}",
                man(y.gross),
                man(salary),
                man(y.gross + salary),
            ),
        );
        text(
            "income-salary-formula",
            format!(
                "給与所得　給与{} − 給与所得控除{} ＝ {}",
                man(salary),
                man(salary - y.salary_income),
                man(y.salary_income),
            ),
        );
        text(
            "income-combined-formula",
            format!(
                "合計所得（{mode}）　配信所得{} ＋ 給与所得{} ＝ {}",
                man(y.business_income),
                man(y.salary_income),
                man(y.total_income),
            ),
        );
    }
    for (id, n) in [
        ("hero-it-2026", y.income_tax),
        ("hero-rt-2026", y.resident_tax),
        ("hero-nhi-2026", y.nhi),
        ("lock-total-amount", out.lock_total),
        ("free-money-amount", out.free_money),
    ] {
        text(id, man(n));
    }
    text(
        "hero-it-sub",
        format!(
            "基礎控除{} / 課税所得{}",
            man(y.basic_deduction),
            man(y.taxable_income)
        ),
    );
    text("hero-rt-sub", &y.resident_tax_label);
    text("hero-nhi-sub", &out.prefecture_name);
    text(
        "reserve-monthly-amount",
        format!("月 約{}", man((out.lock_total + 1199) / 1200 * 100)),
    );
    text("lock-total-exact", yen(out.lock_total));
    text("free-money-exact", yen(out.free_money));
    text("aside-percent", format!("{}%", out.set_aside_percent));
    text("sched-it", format!("約{}（年1回）", man(y.income_tax)));
    text(
        "sched-rt",
        format!(
            "1回 約{}（年4回の目安）",
            man((y.resident_tax + 399) / 400 * 100)
        ),
    );
    text(
        "sched-nhi",
        format!("1回 約{}（年10回の目安）", man((y.nhi + 999) / 1000 * 100)),
    );
    hidden("late-filing-banner", out.late_filing.is_none());
    if let Some(l) = &out.late_filing {
        text("late-filing-msg", &l.message);
        text(
            "late-filing-numbers",
            format!(
                "所得税{} ＋ 加算・延滞税 約{}",
                yen(l.tax),
                yen(l.penalty + l.delinquency)
            ),
        );
    }
    for y in &out.years {
        let yr = y.year;
        for (kind, d) in [("tax", &y.tax_dependent), ("ins", &y.insurance_dependent)] {
            let id = format!("dep-{kind}-badge-{yr}");
            el(&id).set_class_name(&format!("status-badge badge-{}", status(d.status)));
            text(&id, &d.label);
            text(&format!("dep-{kind}-msg-{yr}"), &d.message);
        }
        for (id, n) in [
            ("net", y.business_income),
            ("bd-total", y.total),
            ("bd-it", y.income_tax),
            ("bd-rt", y.resident_tax),
            ("bd-nhi", y.nhi),
            ("bd-pen", y.pension),
        ] {
            text(&format!("{id}-{yr}"), man(n));
        }
        text(&format!("bd-total-exact-{yr}"), yen(y.total));
        text(
            &format!("bd-basic-{yr}"),
            format!("基礎控除 {}", man(y.basic_deduction)),
        );
        text(
            &format!("blue-tag-{yr}"),
            format!("（青色控除 {}）", man(y.blue_deduction)),
        );
    }
    html("action-list",&out.actions.iter().map(|x|format!("<li class='action-item priority-{}'><div class='action-header'><span class='action-icon'>{}</span><span class='action-title'>{}</span><span class='action-when'>{}</span></div><p class='action-detail'>{}</p></li>",status(x.priority),esc(&x.icon),esc(&x.title),esc(&x.when),esc(&x.detail))).collect::<String>());
    html(
        "notes-list",
        &out.notes
            .iter()
            .map(|n| format!("<li>{}</li>", esc(n)))
            .collect::<String>(),
    );
    html(
        "detected-tags",
        &[
            format!("{}歳", s.age),
            out.prefecture_name,
            format!("2025年振込 {}", man(s.income2025)),
            format!("2026年振込 {}", man(s.income2026)),
            format!("今年経費 {}", man(s.expense2026)),
        ]
        .iter()
        .map(|x| format!("<span class='detected-chip'>{}</span>", esc(x)))
        .collect::<String>(),
    );
    render_ledger(a);
}
fn render_ledger(a: &App) {
    let sum = ledger::summarize(&a.entries, a.year);
    active("data-ledger-year", &a.year.to_string());
    for (id, n) in [
        ("lv-total-inc", sum.income),
        ("lv-total-exp", sum.expense),
        ("lv-net-profit", sum.income - sum.expense),
    ] {
        text(id, yen(n));
    }
    html(
        "lv-check-list",
        &format!(
            "<li>{}年の記帳件数: {}件</li><li>{}</li><li>{}</li>",
            a.year,
            a.entries
                .iter()
                .filter(|e| e.date.starts_with(&format!("{}-", a.year)))
                .count(),
            if sum.expense <= sum.income {
                "✅ 売上・経費の集計済み"
            } else {
                "⚠️ 経費が売上を上回っています"
            },
            if a.storage_ok {
                "このブラウザに保存しています。定期的にCSVを保管してください。"
            } else {
                "⚠️ ブラウザへの保存ができません。CSVを保管してください。"
            }
        ),
    );
    html(
        "ledger-cat-tbody",
        &sum.categories
            .iter()
            .map(|(c, n)| format!("<tr><td>{}</td><td>{}</td></tr>", esc(c), yen(*n)))
            .collect::<String>(),
    );
    html(
        "ledger-month-tbody",
        &sum.monthly
            .iter()
            .enumerate()
            .map(|(i, (inc, exp))| {
                format!(
                    "<tr><td>{}月</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    i + 1,
                    yen(*inc),
                    yen(*exp),
                    yen(inc - exp)
                )
            })
            .collect::<String>(),
    );
    let mut es: Vec<_> = a
        .entries
        .iter()
        .filter(|e| e.date.starts_with(&format!("{}-", a.year)))
        .collect();
    es.sort_by(|a, b| a.date.cmp(&b.date));
    html("ledger-entries-tbody",&es.iter().map(|e|format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><button type='button' class='btn-del-entry' data-del-id='{}'>削除</button></td></tr>",esc(&e.date),if e.kind=="income"{"売上"}else{"経費"},esc(&e.category),yen(e.amount),esc(&e.memo),esc(&e.id))).collect::<String>());
}
fn save(a: &mut App) {
    a.storage_ok = web_sys::window()
        .unwrap()
        .local_storage()
        .ok()
        .flatten()
        .is_some_and(|st| {
            st.set_item(ledger::KEY, &serde_json::to_string(&a.entries).unwrap())
                .is_ok()
        });
}
fn feedback(s: &str) {
    hidden("lf-submit-feedback", false);
    text("lf-submit-feedback", s);
}
fn categories(kind: &str) {
    setval("lf-type", kind);
    active("data-lf-type", kind);
    let cats = if kind == "income" {
        ledger::INCOME
    } else {
        ledger::EXPENSE
    };
    html(
        "lf-category",
        &cats
            .iter()
            .map(|x| format!("<option>{}</option>", esc(x)))
            .collect::<String>(),
    );
    text("lf-date-label", "記録する日付（振込・支払日）");
    text(
        "lf-cat-label",
        if kind == "income" {
            "売上の種類"
        } else {
            "経費の種類"
        },
    );
    text(
        "lf-amount-label",
        if kind == "income" {
            "振り込まれた金額（円）"
        } else {
            "支払った経費（円）"
        },
    );
}
fn days() {
    let y = num("lf-year") as i32;
    let m = num("lf-month") as u32;
    let d = num("lf-day").max(1).min(ledger::days(y, m) as i64);
    html(
        "lf-day",
        &(1..=ledger::days(y, m))
            .map(|n| format!("<option value='{n}'>{n}日</option>"))
            .collect::<String>(),
    );
    setval("lf-day", d);
}
fn update(a: &mut App, id: &str) {
    match id {
        "blue-opening-date" => {
            render_blue_deadline();
            return;
        }
        "voice-transcript-input" => parser::apply(&val(id), &mut a.input),
        "pref-select" => a.input.prefecture = num(id) as usize,
        "select-start-2025" => a.input.start_month2025 = num(id) as u32,
        "check-filed-2025" => {
            a.input.filed2025 = el(id).unchecked_ref::<HtmlInputElement>().checked();
            if !a.input.filed2025 {
                a.input.blue_return_2025 = false;
            }
        }
        "check-blue-2025" => {
            a.input.blue_return_2025 =
                a.input.filed2025 && el(id).unchecked_ref::<HtmlInputElement>().checked();
        }
        "check-blue-2026" => {
            a.input.blue_return_2026 = el(id).unchecked_ref::<HtmlInputElement>().checked();
        }
        "slider-inc-2025" => a.input.income2025 = num(id) * 10000,
        "slider-exp-2025" => a.input.expense2025 = num(id) * 10000,
        "slider-inc-2026" => a.input.income2026 = num(id) * 10000,
        "slider-exp-2026" => a.input.expense2026 = num(id) * 10000,
        "slider-salary" => a.input.salary_annual = num(id) * 10000,
        "lf-year" | "lf-month" => days(),
        _ => return,
    }
    render(a);
}
fn add_entry(a: &mut App) {
    if a.storage_protected {
        feedback("保存済みデータの読み込みに失敗したため、上書きを停止しています。");
        return;
    }
    let (y, m, d) = (
        num("lf-year") as i32,
        num("lf-month") as u32,
        num("lf-day") as u32,
    );
    let amount = num("lf-amount");
    if !ledger::valid_date(y, m, d) || amount <= 0 || amount > 1_000_000_000 {
        feedback("日付と金額（1〜10億円の整数）を確認してください。");
        return;
    }
    let memo = val("lf-memo");
    a.entries.push(Entry {
        id: format!("{}-{}", js_sys::Date::now(), a.entries.len()),
        date: format!("{y:04}-{m:02}-{d:02}"),
        kind: val("lf-type"),
        category: val("lf-category"),
        amount,
        memo: if memo.is_empty() {
            val("lf-category")
        } else {
            memo
        },
    });
    save(a);
    a.year = y;
    render_ledger(a);
    setval("lf-amount", "");
    setval("lf-memo", "");
    feedback(if a.storage_ok {
        "✅ 記帳してこのブラウザに保存しました。"
    } else {
        "⚠️ 記帳しましたが保存できません。CSVを保管してください。"
    });
}
fn share(a: &App) {
    let o = diagnose(&a.input);
    let mut s = format!(
        "【ライバー税金・扶養診断】\n{}\n{} / {}歳\n",
        o.headline, o.prefecture_name, a.input.age
    );
    for y in &o.years {
        s += &format!(
            "{}年: 振込{} 経費{} 所得{}\n税・保険等合計{}\n税の扶養: {} / 健康保険: {}\n",
            y.year,
            man(y.gross),
            man(y.expense),
            man(y.business_income),
            man(y.total),
            y.tax_dependent.label,
            y.insurance_dependent.label
        );
    }
    s += &format!(
        "残すお金: {} / 手残り: {}",
        man(o.lock_total),
        man(o.free_money)
    );
    setval("share-modal-textarea", s);
    hidden("share-modal", false);
    text(
        "share-modal-status",
        "「コピー」を押すか、テキストを選択してコピーしてください。",
    );
}
fn download(a: &App) -> Result<(), JsValue> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(&ledger::csv(&a.entries)));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("text/csv;charset=utf-8");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let anchor = doc()
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    anchor.set_href(&url);
    anchor.set_download("tiktok_liver_tax_ledger_all_years.csv");
    anchor.click();
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}
fn click(a: &mut App, e: Element) -> Result<(), JsValue> {
    let id = e.id();
    let attr = |k: &str| e.get_attribute(k);
    if let Some(v) = attr("data-age-group") {
        a.input.age = match v.as_str() {
            "u18" => 17,
            "student" => 20,
            _ => 25,
        };
    } else if let Some(v) = attr("data-has-salary") {
        a.input.has_salary = v == "true";
        if a.input.has_salary && a.input.salary_annual == 0 {
            a.input.salary_annual = 800000;
        }
    } else if let Some(v) = attr("data-voice-sample") {
        setval("voice-transcript-input", &v);
        parser::apply(&v, &mut a.input);
    } else if let Some(v) = attr("data-quick-2025") {
        a.input.income2025 = v.parse::<i64>().unwrap_or(0) * 10000;
    } else if let Some(v) = attr("data-quick-2026") {
        a.input.income2026 = v.parse::<i64>().unwrap_or(0) * 10000;
    } else if let Some(v) = attr("data-step-target") {
        let delta = attr("data-step-delta")
            .and_then(|x| x.parse::<i64>().ok())
            .unwrap_or(0);
        let max = el(&v)
            .get_attribute("max")
            .and_then(|x| x.parse::<i64>().ok())
            .unwrap_or(1000);
        setval(&v, (num(&v) + delta).clamp(0, max));
        update(a, &v);
        return Ok(());
    } else if let Some(v) = attr("data-lf-type") {
        categories(&v);
        return Ok(());
    } else if let Some(v) = attr("data-ledger-year") {
        a.year = v.parse().unwrap_or(2026);
        render_ledger(a);
        return Ok(());
    } else if let Some(v) = attr("data-del-id") {
        if web_sys::window()
            .unwrap()
            .confirm_with_message("この台帳記録を削除しますか？")?
        {
            a.entries.retain(|e| e.id != v);
            save(a);
            render_ledger(a);
        }
        return Ok(());
    } else if let Some(v) = attr("data-amt-add") {
        setval(
            "lf-amount",
            num("lf-amount") + v.parse::<i64>().unwrap_or(0),
        );
        return Ok(());
    } else if attr("data-amt-clear").is_some() {
        setval("lf-amount", "");
        return Ok(());
    } else if let Some(v) = attr("data-date-preset") {
        let now = js_sys::Date::new_0();
        let (mut y, mut m) = (num("lf-year"), num("lf-month"));
        match v.as_str() {
            "today" => {
                y = now.get_full_year().clamp(2025, 2026) as i64;
                m = now.get_month() as i64 + 1;
            }
            "2025-end" => {
                y = 2025;
                m = 12;
            }
            "last-month-end" => {
                m -= 1;
                if m == 0 {
                    m = 12;
                    y = 2025;
                }
            }
            _ => {}
        }
        setval("lf-year", y);
        setval("lf-month", m);
        days();
        setval(
            "lf-day",
            if v == "today" {
                now.get_date()
            } else {
                ledger::days(y as i32, m as u32)
            },
        );
        return Ok(());
    } else if let Some(v) = attr("data-help-key") {
        let id = format!("lf-pop-{v}");
        let show = el(&id).class_list().contains("hidden");
        each(".lf-inline-popover", |e| class(&e, "hidden", true));
        text(&id,match v.as_str(){"type"=>"売上は振込報酬、経費は配信業務に必要な支出を記録します。","date"=>"年・月・日を選択してください。月まとめの場合も証憑と対応できるように保管します。","category"=>"売上または経費の科目を選ぶと、年度ごとに自動集計します。","amount"=>"1円単位の整数で入力してください。私用分は経費から除きます。",_=>"後で確認できる内容や用途を記録してください。"});
        hidden(&id, !show);
        return Ok(());
    } else {
        match id.as_str() {
            "btn-blue-application" => {
                render_blue_deadline();
                return el("blue-application-dialog")
                    .unchecked_ref::<HtmlDialogElement>()
                    .show_modal();
            }
            "btn-blue-application-close" => {
                el("blue-application-dialog")
                    .unchecked_ref::<HtmlDialogElement>()
                    .close();
                return Ok(());
            }
            "mic-btn" => {
                toggle_speech(a)?;
                return Ok(());
            }
            "btn-sync-ledger-to-diag" => {
                let x = ledger::summarize(&a.entries, 2025);
                let y = ledger::summarize(&a.entries, 2026);
                a.input.income2025 = x.income;
                a.input.expense2025 = x.expense;
                a.input.income2026 = y.income;
                a.input.expense2026 = y.expense;
                el("result-card").scroll_into_view();
            }
            "btn-export-csv" => return download(a),
            "btn-copy-line" => {
                share(a);
                return Ok(());
            }
            "btn-modal-copy-again" => {
                let _ = web_sys::window()
                    .unwrap()
                    .navigator()
                    .clipboard()
                    .write_text(&val("share-modal-textarea"));
                el("share-modal-textarea")
                    .unchecked_ref::<HtmlTextAreaElement>()
                    .select();
                text("share-modal-status","コピーできない場合は、選択したテキストを長押し、または Command+C でコピーしてください。");
                return Ok(());
            }
            "btn-modal-close-bottom" | "share-modal-backdrop" => {
                hidden("share-modal", true);
                return Ok(());
            }
            "btn-screenshot-mode" => {
                let body = doc().body().unwrap();
                let shot = body.class_list().toggle("screenshot-view")?;
                text(
                    "btn-screenshot-mode",
                    if shot {
                        "✏️ 入力画面に戻る"
                    } else {
                        "📸 スクショ1画面"
                    },
                );
                if shot {
                    el("result-card").scroll_into_view();
                }
                return Ok(());
            }
            _ => return Ok(()),
        }
    }
    render(a);
    Ok(())
}
fn get(o: &JsValue, k: &str) -> JsValue {
    js_sys::Reflect::get(o, &k.into()).unwrap_or(JsValue::UNDEFINED)
}
fn put(o: &JsValue, k: &str, v: &JsValue) {
    let _ = js_sys::Reflect::set(o, &k.into(), v);
}
fn call(o: &JsValue, k: &str) -> Result<JsValue, JsValue> {
    get(o, k).dyn_into::<js_sys::Function>()?.call0(o)
}
fn speech_ui(on: bool) {
    class(&el("mic-btn"), "listening", on);
    text("mic-emoji", if on { "🛑" } else { "🎙️" });
    text(
        "mic-label-main",
        if on {
            "聞き取り中… 話し終わったらタップ"
        } else {
            "タップして状況を喋る"
        },
    );
    hidden("mic-level-wrap", true);
}
fn toggle_speech(a: &mut App) -> Result<(), JsValue> {
    if a.listening {
        if let Some(r) = &a.speech {
            let _ = call(r, "stop");
        }
        a.listening = false;
        speech_ui(false);
        return Ok(());
    }
    if let Some(r) = &a.speech {
        call(r, "start")?;
        a.listening = true;
        speech_ui(true);
        return Ok(());
    }
    let win = JsValue::from(web_sys::window().unwrap());
    let mut ctor = get(&win, "SpeechRecognition");
    if ctor.is_undefined() {
        ctor = get(&win, "webkitSpeechRecognition");
    }
    if !ctor.is_function() {
        el("voice-transcript-input")
            .unchecked_ref::<web_sys::HtmlElement>()
            .focus()?;
        text("mic-status","ブラウザの音声認識に非対応です。キーボードの音声入力、例文、テキスト入力をご利用ください。");
        return Ok(());
    }
    let r = js_sys::Reflect::construct(
        &ctor.unchecked_into::<js_sys::Function>(),
        &js_sys::Array::new(),
    )?;
    put(&r, "lang", &"ja-JP".into());
    put(&r, "continuous", &true.into());
    put(&r, "interimResults", &true.into());
    let result = Closure::<dyn FnMut(JsValue)>::new(|event: JsValue| {
        let results = get(&event, "results");
        let len = get(&results, "length").as_f64().unwrap_or(0.) as u32;
        let mut s = String::new();
        for i in 0..len {
            let item = get(&results, &i.to_string());
            let alt = get(&item, "0");
            s += &get(&alt, "transcript").as_string().unwrap_or_default();
            s.push('、');
        }
        APP.with(|a| {
            let mut a = a.borrow_mut();
            setval("voice-transcript-input", &s);
            parser::apply(&s, &mut a.input);
            render(&a);
        });
    });
    put(&r, "onresult", result.as_ref());
    result.forget();
    let error = Closure::<dyn FnMut(JsValue)>::new(|event: JsValue| {
        APP.with(|a| a.borrow_mut().listening = false);
        speech_ui(false);
        text("mic-status",format!("音声認識を利用できません（{}）。テキスト欄やキーボードの音声入力をお使いください。",get(&event,"error").as_string().unwrap_or_default()));
    });
    put(&r, "onerror", error.as_ref());
    error.forget();
    let end = Closure::<dyn FnMut()>::new(|| {
        APP.with(|a| a.borrow_mut().listening = false);
        speech_ui(false);
    });
    put(&r, "onend", end.as_ref());
    end.forget();
    call(&r, "start")?;
    a.speech = Some(r);
    a.listening = true;
    speech_ui(true);
    text(
        "mic-status",
        "聞き取り中です。停止するにはもう一度タップしてください。",
    );
    Ok(())
}
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    el("app").set_inner_html(include_str!("view.html"));
    text(
        "engine-mode-badge",
        "🦀 画面・台帳・診断すべて Rust / WebAssembly",
    );
    // Speech recognition can use the browser vendor's service; calculations and ledger remain local.
    each(".privacy-pill", |e| {
        e.set_text_content(Some("🔒 計算・台帳保存はこの端末内。音声認識はブラウザ提供サービスを使用する場合があります。"))
    });
    html(
        "pref-select",
        &rust_tax_engine::prefectures::NHI_TABLE
            .iter()
            .enumerate()
            .map(|(i, p)| format!("<option value='{i}'>{}（{}）</option>", p.name, p.city))
            .collect::<String>(),
    );
    html(
        "lf-month",
        &(1..=12)
            .map(|n| format!("<option value='{n}'>{n}月</option>"))
            .collect::<String>(),
    );
    let now = js_sys::Date::new_0();
    setval("lf-year", now.get_full_year().clamp(2025, 2026));
    setval("lf-month", now.get_month() + 1);
    days();
    setval("lf-day", now.get_date());
    categories("income");
    APP.with(|app| {
        let mut a = app.borrow_mut();
        a.year = 2026;
        a.input.income2025 = 500000;
        a.input.income2026 = 2500000;
        a.input.expense2026 = 300000;
        a.input.start_month2025 = 10;
        let deadline = js_sys::Date::new(&"2026-03-16T00:00:00+09:00".into()).get_time();
        a.input.days_since_2025_deadline = ((js_sys::Date::now() - deadline) / 86400000.).floor().max(0.) as i64;
        let storage = web_sys::window().unwrap().local_storage().ok().flatten();
        a.storage_ok = storage.is_some();
        match storage.and_then(|s| s.get_item(ledger::KEY).ok().flatten()) {
            Some(raw) => match serde_json::from_str::<Vec<Entry>>(&raw) {
                Ok(entries) => a.entries = entries,
                Err(_) => {
                    a.storage_protected = true;
                    a.storage_ok = false;
                    feedback("⚠️ 保存データを読み込めません。既存データを保護するため記帳を停止しています。");
                }
            },
            None => a.entries = serde_json::from_str(include_str!("sample-ledger.json")).unwrap(),
        }
        render(&a);
    });
    for kind in ["click", "input", "change", "submit"] {
        let k = kind.to_string();
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            if k == "submit" {
                if target.id() == "ledger-form" {
                    event.prevent_default();
                    APP.with(|a| add_entry(&mut a.borrow_mut()));
                }
                return;
            }
            if k == "click" {
                let e = target
                    .closest("button, #share-modal-backdrop")
                    .ok()
                    .flatten();
                if let Some(e) = e {
                    APP.with(|a|{if let Err(err)=click(&mut a.borrow_mut(),e){web_sys::console::error_1(&err);text("mic-status","操作を完了できませんでした。入力内容とブラウザの権限を確認してください。");}});
                }
            } else {
                APP.with(|a| update(&mut a.borrow_mut(), &target.id()));
            }
        });
        doc().add_event_listener_with_callback(kind, cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    // Native <details> controls keep working without rebuilding the form.
    let toggle = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        if let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if target.tag_name() == "DETAILS" {
                if let Ok(Some(hint)) = target.query_selector("[data-open-text]") {
                    let key = if target.has_attribute("open") {
                        "data-close-text"
                    } else {
                        "data-open-text"
                    };
                    hint.set_text_content(hint.get_attribute(key).as_deref());
                }
            }
        }
    });
    doc().add_event_listener_with_callback_and_bool(
        "toggle",
        toggle.as_ref().unchecked_ref(),
        true,
    )?;
    toggle.forget();
    Ok(())
}
