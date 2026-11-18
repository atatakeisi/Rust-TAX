mod actions_ui;
mod blue_deadline;
mod guide_dialog;
mod guidance;
mod ledger;
mod ledger_wizard;
mod parser;
use guidance::{Checked, Topic};
use ledger::Entry;
use rust_tax_engine::{
    diagnose::diagnose,
    model::{DiagnoseInput, DiagnoseOutput, HealthCoverage, Status},
};
use std::cell::RefCell;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{
    Element, FocusOptions, HtmlDialogElement, HtmlElement, HtmlInputElement, HtmlSelectElement,
    HtmlTextAreaElement,
};
const YEN_PER_MAN: i64 = 10_000;
thread_local! {static APP:RefCell<App>=RefCell::new(App::default());}
#[derive(Default)]
struct App {
    input: DiagnoseInput,
    entries: Vec<Entry>,
    year: i32,
    storage_ok: bool,
    storage_protected: bool,
    ledger_step: ledger_wizard::Step,
    speech: Option<JsValue>,
    speech_state: SpeechState,
    demo: Option<DemoBackup>,
    checked: Checked,
    guidance_storage_ok: bool,
    guide_dialog: Option<guide_dialog::State>,
}

struct DemoBackup {
    input: DiagnoseInput,
    transcript: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SpeechState {
    #[default]
    Idle,
    Listening,
    Stopping,
}
fn doc() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
fn el(id: &str) -> Element {
    doc()
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("missing element {id}"))
}
fn focus_without_scroll(element: &Element) -> Result<(), JsValue> {
    let options = FocusOptions::new();
    options.set_prevent_scroll(true);
    element.unchecked_ref::<HtmlElement>().focus_with_options(&options)
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
fn open_blue_dialog() -> Result<(), JsValue> {
    render_blue_deadline();
    el("blue-application-dialog")
        .unchecked_ref::<HtmlDialogElement>()
        .show_modal()
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
        let slider = el(&format!("slider-{id}")).unchecked_into::<HtmlInputElement>();
        let thumb = n.saturating_add(YEN_PER_MAN / 2).max(0) / YEN_PER_MAN;
        let max = slider
            .get_attribute("max")
            .and_then(|x| x.parse::<i64>().ok())
            .unwrap_or(0);
        if thumb > max {
            slider.set_max(&(thumb + 100).to_string());
        }
        let position = thumb.to_string();
        if slider.value() != position {
            slider.set_value(&position);
        }
        let _ = slider.set_attribute("aria-valuetext", &yen(n));
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
    render_estimate(a);
    render_ledger(a);
}
fn render_estimate(a: &App) {
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
    let mut scenario = s.clone();
    scenario.blue_return_2026 = !s.blue_return_2026;
    let alternate = diagnose(&scenario);
    let (white, blue) = if s.blue_return_2026 {
        (&alternate.years[1], y)
    } else {
        (y, &alternate.years[1])
    };
    for (column, values) in [("white", white), ("blue", blue)] {
        for (row, amount) in [
            ("income", values.business_income),
            ("it", values.income_tax),
            ("rt", values.resident_tax),
            ("nhi", values.nhi),
        ] {
            text(&format!("compare-{column}-{row}"), man(amount));
        }
    }
    text(
        "income-white-equation",
        format!("振込{} − 経費{} ＝ ", man(y.gross), man(y.expense)),
    );
    text(
        "income-blue-equation",
        format!(
            "振込{} − 経費{} − 青色控除{} ＝ ",
            man(blue.gross),
            man(blue.expense),
            man(blue.blue_deduction),
        ),
    );
    let mode = if s.blue_return_2026 { "青色" } else { "白色" };
    text("detail-mode", format!("2026年分の詳細（{mode}）"));
    render_guidance(a, &out);
    let salary = s.salary_annual.max(0);
    text("result-income", man(y.gross));
    text("result-expense", man(y.expense));
    text("result-salary", man(salary));
    hidden("result-salary-item", !s.has_salary);
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
        ("lock-total-amount", y.total),
        ("free-money-amount", out.free_money),
    ] {
        text(id, man(n));
    }
    text(
        "income-tax-basis",
        format!(
            "{mode}の所得税　基礎控除 {}・課税所得 {}",
            man(y.basic_deduction),
            man(y.taxable_income)
        ),
    );
    text(
        "reserve-monthly-amount",
        format!("月 約{}", man((y.total + 1199) / 1200 * 100)),
    );
    text("lock-total-exact", yen(y.total));
    text("free-money-exact", yen(out.free_money));
    text("sched-it", man(y.income_tax));
    text("sched-rt", man(y.resident_tax));
    text("sched-nhi", man(y.nhi));
    text("sched-pension", man(y.pension));
    hidden("late-filing-banner", out.late_filing.is_none());
    if let Some(l) = &out.late_filing {
        text("late-filing-msg", &l.message);
        text(
            "late-filing-numbers",
            format!(
                "試算：所得税{} ＋ 加算・延滞税 約{}",
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
    actions_ui::render(&out.actions);
    html(
        "notes-list",
        &out.notes
            .iter()
            .map(|n| format!("<li>{}</li>", esc(n)))
            .collect::<String>(),
    );
}
fn render_guidance(a: &App, out: &DiagnoseOutput) {
    let demo = a.demo.is_some();
    let entered = guidance::has_amounts(&a.input);
    hidden("demo-indicator", !demo);
    hidden("btn-end-demo", !demo);
    hidden("adjust-demo-notice", !demo);
    hidden("guide-prior-demo-note", !demo);
    hidden("guide-ledger-demo-note", !demo);
    let _ = el("guide-slot-ledger").toggle_attribute_with_force("disabled", demo);
    let _ = el("btn-sync-ledger-in-guide").toggle_attribute_with_force("disabled", demo);
    hidden("btn-try-guide", demo || entered);
    hidden("guide-demo-row", entered && !demo);
    hidden("next-step", !entered);
    hidden("drawer-next-steps", !entered);

    let coverage = match a.input.health_coverage_2026 {
        HealthCoverage::Unknown if a.input.salary_has_shaho => "employer",
        HealthCoverage::Unknown => "unknown",
        HealthCoverage::Dependent => "dependent",
        HealthCoverage::Employer => "employer",
        HealthCoverage::National => "national",
    };
    hidden("insurance-estimate-note", coverage == "unknown");
    setval("guide-health-coverage", coverage);
    let steps = guidance::steps(&a.input, out);
    let mut next = None;
    let mut checked_count = 0;
    for step in &steps {
        let id = step.topic.id();
        let checked = !demo
            && a.checked.get(&step.topic).is_some_and(|scope| scope == step.review_scope);
        checked_count += usize::from(checked);
        text(&format!("guide-title-{id}"), step.title);
        text(&format!("guide-copy-{id}"), format!("{} {}", step.preview, step.detail));
        let status_id = format!("guide-status-{id}");
        let (label, tone) = if checked {
            ("確認済み", "safe")
        } else if step.priority >= 4 {
            ("先に確認", "caution")
        } else {
            ("", "")
        };
        hidden(&status_id, label.is_empty());
        if !label.is_empty() {
            el(&status_id).set_class_name(&format!("status-badge badge-{tone}"));
            text(&status_id, label);
        }
        let check_id = format!("guide-check-{id}");
        hidden(&check_id, demo);
        text(&check_id, if checked { "確認の記録を戻す" } else { "確認済みにする" });
        if !checked && step.priority > 0 && next.is_none_or(|current: &guidance::Step| step.priority > current.priority) {
            next = Some(step);
        }
    }
    text("guidance-count", if demo { "入力例".to_owned() } else { format!("確認 {checked_count} / {}", Topic::ALL.len()) });
    text("guidance-note", if demo {
        "入力例で確認する順番を試せます。"
    } else if !a.guidance_storage_ok {
        "確認の記録は画面を閉じるまで残ります。"
    } else {
        "確認の記録はこの端末に保存されます。収入や加入先が変わったら見直してください。"
    });
    hidden("next-step-badge", !demo);
    if demo {
        text("next-step-badge", "入力例");
        el("next-step-badge").set_class_name("status-badge badge-caution");
    }
    let (page, target, title, preview) = next.map_or(
        (guide_dialog::Page::Ledger, "ledger", "振込や経費を台帳に記録する", "振込明細や領収書と一緒に、まず1件だけ残しましょう。"),
        |step| (guide_dialog::Page::Topic(step.topic), step.topic.id(), step.title, step.preview),
    );
    let (category, label) = page.category();
    let _ = el("next-step").set_attribute("data-guide-category", category);
    text("next-step-category", label);
    text("next-step-copy", preview);
    text("btn-next-step", title);
    let _ = el("btn-next-step").set_attribute("data-guide-open", target);
    guide_dialog::update_title(a);
}
fn save_guidance(a: &mut App) {
    if a.guidance_storage_ok {
        a.guidance_storage_ok = serde_json::to_string(&a.checked)
            .ok()
            .and_then(|data| {
                web_sys::window()?.local_storage().ok().flatten()?
                    .set_item(guidance::STORAGE_KEY, &data).ok()
            })
            .is_some();
    }
}
fn end_demo(a: &mut App) {
    if let Some(backup) = a.demo.take() {
        a.input = backup.input;
        setval("voice-transcript-input", backup.transcript);
    }
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
fn feedback(message: &str, success: bool) {
    let feedback = el("lf-submit-feedback");
    class(&feedback, "hidden", false);
    class(&feedback, "is-success", success);
    class(&feedback, "is-error", !success);
    feedback.set_text_content(Some(message));
}
fn categories(kind: &str) {
    let chosen = val("lf-category");
    let same_kind = val("lf-type") == kind;
    setval("lf-type", kind);
    active("data-lf-type", kind);
    each("[data-lf-type]", |button| {
        let pressed = button.get_attribute("data-lf-type").as_deref() == Some(kind);
        let _ = button.set_attribute("aria-pressed", if pressed { "true" } else { "false" });
    });
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
    if same_kind && cats.contains(&chosen.as_str()) {
        setval("lf-category", chosen);
    }
    text(
        "ledger-heading-date",
        if kind == "income" {
            "いつ振り込まれましたか？"
        } else {
            "いつ支払いましたか？"
        },
    );
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
    each("[data-date-preset='last-month-end']", |button| {
        let _ = button.toggle_attribute_with_force("disabled", y == 2025 && m == 1);
    });
}
fn slider_amount<'a>(input: &'a mut DiagnoseInput, slider: &str) -> Option<&'a mut i64> {
    match slider {
        "slider-inc-2025" => Some(&mut input.income2025),
        "slider-exp-2025" => Some(&mut input.expense2025),
        "slider-inc-2026" => Some(&mut input.income2026),
        "slider-exp-2026" => Some(&mut input.expense2026),
        "slider-salary" => Some(&mut input.salary_annual),
        _ => None,
    }
}
fn update(a: &mut App, id: &str) {
    if let Some(amount) = slider_amount(&mut a.input, id) {
        let next = num(id).saturating_mul(YEN_PER_MAN);
        if *amount != next {
            *amount = next;
            render_estimate(a);
        }
        return;
    }
    match id {
        "guide-health-coverage" => {
            let coverage = match val(id).as_str() {
                "dependent" => HealthCoverage::Dependent,
                "employer" => HealthCoverage::Employer,
                "national" => HealthCoverage::National,
                _ => HealthCoverage::Unknown,
            };
            a.input.health_coverage_2026 = coverage;
        }
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
        "lf-year" | "lf-month" => return days(),
        _ => return,
    }
    render_estimate(a);
}
fn add_entry(a: &mut App) -> Result<(), JsValue> {
    let (y, m, d) = ledger_wizard::date();
    let amount = num("lf-amount");
    if !ledger::valid_date(y, m, d) || amount <= 0 || amount > 1_000_000_000 {
        feedback("日付と金額（1〜10億円の整数）を確認してください。", false);
        return Ok(());
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
    ledger_wizard::restart(a)?;
    if a.storage_ok {
        feedback("✅ 記帳してこのブラウザに保存しました。", true);
    } else {
        feedback("⚠️ 記帳しましたが保存できません。CSVを保管してください。", false);
    }
    Ok(())
}
fn share(a: &App) {
    let o = diagnose(&a.input);
    let mut s = format!(
        "【ライバー税金・扶養診断{}】\n{}\n{} / {}歳\n",
        if a.demo.is_some() { "（入力例）" } else { "" },
        o.headline,
        o.prefecture_name,
        a.input.age,
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
        "2026年分の負担目安: {} / 配信の手残り目安: {}",
        man(o.years[1].total),
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
    if let Some(result) = actions_ui::click(&e) {
        return result;
    }
    let id = e.id();
    let attr = |k: &str| e.get_attribute(k);
    if let Some(topic) = attr("data-guide-open") {
        if topic == "ledger" {
            return guide_dialog::open(a, guide_dialog::Page::Ledger, e);
        }
        if let Some(topic) = Topic::ALL.into_iter().find(|item| item.id() == topic) {
            return guide_dialog::open(a, guide_dialog::Page::Topic(topic), e);
        }
    } else if let Some(target) = attr("data-guide-target") {
        let page = match target.as_str() {
            "prior-return" => guide_dialog::Page::PriorInput,
            "ledger" => guide_dialog::Page::Ledger,
            "blue" => guide_dialog::Page::BlueDeadline,
            _ => return Ok(()),
        };
        return guide_dialog::open(a, page, e);
    } else if let Some(topic) = attr("data-guide-check") {
        if a.demo.is_none() {
            let out = diagnose(&a.input);
            if let Some(step) = guidance::steps(&a.input, &out)
                .into_iter()
                .find(|step| step.topic.id() == topic)
            {
                if a.checked.get(&step.topic).is_some_and(|scope| scope == step.review_scope) {
                    a.checked.remove(&step.topic);
                } else {
                    a.checked.insert(step.topic, step.review_scope.to_owned());
                }
                save_guidance(a);
            }
        }
    } else if let Some(v) = attr("data-age-group") {
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
        if a.demo.is_none() {
            a.demo = Some(DemoBackup { input: a.input.clone(), transcript: val("voice-transcript-input") });
        }
        a.input = DiagnoseInput {
            days_since_2025_deadline: a.input.days_since_2025_deadline,
            ..DiagnoseInput::default()
        };
        setval("voice-transcript-input", &v);
        parser::apply(&v, &mut a.input);
        mic_status("");
        render(a);
        return show_comparison();
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
        if let Some(amount) = slider_amount(&mut a.input, &v) {
            let next = amount
                .saturating_add(delta.saturating_mul(YEN_PER_MAN))
                .clamp(0, max.saturating_mul(YEN_PER_MAN));
            if *amount != next {
                *amount = next;
                render_estimate(a);
            }
        }
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
                    if y == 2025 {
                        return Ok(());
                    }
                    m = 12;
                    y -= 1;
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
        ledger_wizard::close_help();
        text(&id,match v.as_str(){"type"=>"売上は振込報酬、経費は配信業務に必要な支出を記録します。","date"=>"年・月・日を選択してください。月まとめの場合も証憑と対応できるように保管します。","category"=>"売上または経費の科目を選ぶと、年度ごとに自動集計します。","amount"=>"1円単位の整数で入力してください。私用分は経費から除きます。",_=>"後で確認できる内容や用途を記録してください。"});
        hidden(&id, !show);
        e.set_attribute("aria-expanded", if show { "true" } else { "false" })?;
        return Ok(());
    } else {
        match id.as_str() {
            "btn-ledger-back" => return ledger_wizard::back(a),
            "btn-ledger-next" => return ledger_wizard::advance(a),
            "guide-dialog-close" => return guide_dialog::close(a),
            "guide-dialog-back" => return guide_dialog::back(a),
            "btn-end-demo" | "btn-end-demo-adjust" => {
                end_demo(a);
                render(a);
                return el("mic-btn").unchecked_ref::<web_sys::HtmlElement>().focus();
            }
            "btn-end-demo-prior" | "btn-end-demo-ledger" => {
                end_demo(a);
                render_estimate(a);
                return el("guide-dialog-title").unchecked_ref::<web_sys::HtmlElement>().focus();
            }
            "btn-show-result" => {
                mic_status("");
                return show_comparison();
            }
            "btn-blue-application" => return open_blue_dialog(),
            "btn-blue-application-close" => {
                el("blue-application-dialog")
                    .unchecked_ref::<HtmlDialogElement>()
                    .close();
                return Ok(());
            }
            "mic-btn" => {
                if a.demo.is_some() {
                    end_demo(a);
                    render(a);
                }
                toggle_speech(a)?;
                return Ok(());
            }
            "btn-sync-ledger-to-diag" | "btn-sync-ledger-in-guide" => {
                let in_guide = id == "btn-sync-ledger-in-guide";
                if in_guide
                    && (a.demo.is_some()
                        || guide_dialog::current(a) != Some(guide_dialog::Page::Ledger))
                {
                    return Ok(());
                }
                end_demo(a);
                let x = ledger::summarize(&a.entries, 2025);
                let y = ledger::summarize(&a.entries, 2026);
                a.input.income2025 = x.income;
                a.input.expense2025 = x.expense;
                a.input.income2026 = y.income;
                a.input.expense2026 = y.expense;
                if in_guide {
                    render(a);
                    return guide_dialog::close(a);
                }
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
fn set_speech_state(a: &mut App, state: SpeechState) {
    a.speech_state = state;
    let button = el("mic-btn");
    let listening = state == SpeechState::Listening;
    class(&button, "listening", listening);
    let _ = button.toggle_attribute_with_force("disabled", state == SpeechState::Stopping);
    text("mic-emoji", if listening { "🛑" } else { "🎙️" });
    text(
        "mic-label-main",
        match state {
            SpeechState::Idle => "タップして状況を喋る（録音）",
            SpeechState::Listening => "聞き取り中… タップで停止",
            SpeechState::Stopping => "聞き取りを終了中…",
        },
    );
}
fn mic_status(message: &str) {
    text("mic-status", message);
    hidden("mic-status", message.is_empty());
}
fn show_comparison() -> Result<(), JsValue> {
    el("drawer-adjust").remove_attribute("open")?;
    el("comparison-title").unchecked_ref::<web_sys::HtmlElement>().focus()
}
fn stop_speech(a: &mut App) -> Result<(), JsValue> {
    if a.speech_state == SpeechState::Listening {
        if let Some(recognition) = &a.speech {
            call(recognition, "stop").or_else(|_| call(recognition, "abort"))?;
            set_speech_state(a, SpeechState::Stopping);
        }
    }
    Ok(())
}
fn toggle_speech(a: &mut App) -> Result<(), JsValue> {
    if a.speech_state != SpeechState::Idle {
        stop_speech(a)?;
        return Ok(());
    }
    if let Some(r) = &a.speech {
        call(r, "start")?;
        set_speech_state(a, SpeechState::Listening);
        mic_status("");
        return Ok(());
    }
    let win = JsValue::from(web_sys::window().unwrap());
    let mut ctor = get(&win, "SpeechRecognition");
    if ctor.is_undefined() {
        ctor = get(&win, "webkitSpeechRecognition");
    }
    if !ctor.is_function() {
        el("drawer-adjust").set_attribute("open", "")?;
        el("voice-transcript-input")
            .unchecked_ref::<web_sys::HtmlElement>()
            .focus()?;
        mic_status("このブラウザでは、下の欄に入力してください。");
        return Ok(());
    }
    let r = js_sys::Reflect::construct(
        &ctor.unchecked_into::<js_sys::Function>(),
        &js_sys::Array::new(),
    )?;
    put(&r, "lang", &"ja-JP".into());
    put(&r, "continuous", &false.into());
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
        APP.with(|a| set_speech_state(&mut a.borrow_mut(), SpeechState::Idle));
        let error = get(&event, "error").as_string();
        if error.as_deref() == Some("aborted") {
            return;
        }
        let _ = el("drawer-adjust").set_attribute("open", "");
        if error.as_deref() == Some("not-allowed") {
            mic_status("マイクが許可されていません。下の欄からも入力できます。");
        } else {
            mic_status("音声を認識できませんでした。下の欄からも入力できます。");
        }
    });
    put(&r, "onerror", error.as_ref());
    error.forget();
    let end = Closure::<dyn FnMut()>::new(|| {
        APP.with(|a| set_speech_state(&mut a.borrow_mut(), SpeechState::Idle));
    });
    put(&r, "onend", end.as_ref());
    end.forget();
    call(&r, "start")?;
    a.speech = Some(r);
    set_speech_state(a, SpeechState::Listening);
    mic_status("");
    Ok(())
}
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    el("app").set_inner_html(include_str!("view.html"));
    actions_ui::init()?;
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
        let deadline = js_sys::Date::new(&"2026-03-16T00:00:00+09:00".into()).get_time();
        a.input.days_since_2025_deadline = ((js_sys::Date::now() - deadline) / 86400000.).floor().max(0.) as i64;
        let storage = web_sys::window().unwrap().local_storage().ok().flatten();
        a.storage_ok = storage.is_some();
        if let Some(storage) = &storage {
            match storage.get_item(guidance::STORAGE_KEY) {
                Ok(None) => a.guidance_storage_ok = true,
                Ok(Some(raw)) => {
                    if let Ok(checked) = serde_json::from_str(&raw) {
                        a.checked = checked;
                        a.guidance_storage_ok = true;
                    }
                }
                Err(_) => {}
            }
        }
        match storage.and_then(|s| s.get_item(ledger::KEY).ok().flatten()) {
            Some(raw) => match serde_json::from_str::<Vec<Entry>>(&raw) {
                Ok(entries) => a.entries = entries,
                Err(_) => {
                    a.storage_protected = true;
                    a.storage_ok = false;
                    feedback("⚠️ 保存データを読み込めません。既存データを保護するため記帳を停止しています。", false);
                }
            },
            None => {}
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
                    APP.with(|a| {
                        if let Err(error) = ledger_wizard::advance(&mut a.borrow_mut()) {
                            web_sys::console::error_1(&error);
                        }
                    });
                }
                return;
            }
            if k == "click" {
                let e = target
                    .closest("button, #share-modal-backdrop")
                    .ok()
                    .flatten();
                if let Some(e) = e {
                    APP.with(|a|{if let Err(err)=click(&mut a.borrow_mut(),e){web_sys::console::error_1(&err);mic_status("操作を完了できませんでした。");}});
                }
            } else {
                APP.with(|a| update(&mut a.borrow_mut(), &target.id()));
            }
        });
        doc().add_event_listener_with_callback(kind, cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    let guide_closed = Closure::<dyn FnMut()>::new(|| {
        if !guide_dialog::element().open() {
            APP.with(|app| {
                if let Err(error) = guide_dialog::finish(&mut app.borrow_mut()) {
                    web_sys::console::error_1(&error);
                }
            });
        }
    });
    guide_dialog::element().add_event_listener_with_callback(
        "close",
        guide_closed.as_ref().unchecked_ref(),
    )?;
    guide_closed.forget();
    let visibility_change = Closure::<dyn FnMut()>::new(|| {
        if doc().hidden() {
            APP.with(|app| {
                if let Err(error) = stop_speech(&mut app.borrow_mut()) {
                    web_sys::console::error_1(&error);
                }
            });
        }
    });
    doc().add_event_listener_with_callback(
        "visibilitychange",
        visibility_change.as_ref().unchecked_ref(),
    )?;
    visibility_change.forget();
    Ok(())
}
