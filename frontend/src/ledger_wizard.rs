use crate::{
    add_entry, doc, each, el, feedback, focus_without_scroll, guide_dialog, hidden, ledger, num,
    text, val, yen, App,
};
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Step {
    #[default]
    Kind,
    Date,
    Category,
    Amount,
    Review,
}

impl Step {
    const ALL: [Self; 5] = [
        Self::Kind,
        Self::Date,
        Self::Category,
        Self::Amount,
        Self::Review,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::Kind => "kind",
            Self::Date => "date",
            Self::Category => "category",
            Self::Amount => "amount",
            Self::Review => "review",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

pub(super) fn date() -> (i32, u32, u32) {
    (
        num("lf-year") as i32,
        num("lf-month") as u32,
        num("lf-day") as u32,
    )
}

pub(super) fn close_help() {
    each("#ledger-form .lf-inline-popover", |popover| {
        crate::class(&popover, "hidden", true)
    });
    each("#ledger-form [data-help-key]", |button| {
        let _ = button.set_attribute("aria-expanded", "false");
    });
}

fn display(a: &mut App, step: Step) -> Result<(), JsValue> {
    close_help();
    a.ledger_step = step;
    for pane in Step::ALL {
        hidden(&format!("ledger-step-{}", pane.id()), pane != step);
    }
    text(
        "ledger-progress-label",
        format!("ステップ {} / {}", step.index() + 1, Step::ALL.len()),
    );
    if let Some(previous) = doc().query_selector(".ledger-progress-track [aria-current]")? {
        previous.remove_attribute("aria-current")?;
    }
    if let Some(current) = doc().query_selector(&format!(
        ".ledger-progress-track li:nth-child({})",
        step.index() + 1
    ))? {
        current.set_attribute("aria-current", "step")?;
    }
    hidden("btn-ledger-back", step == Step::Kind);
    hidden("btn-ledger-next", step == Step::Review);
    hidden("btn-ledger-submit", step != Step::Review);
    text(
        "btn-ledger-next",
        if step == Step::Amount {
            "入力内容を確認する"
        } else {
            "次へ"
        },
    );
    if step == Step::Review {
        let (year, month, day) = date();
        text(
            "lf-review-kind",
            if val("lf-type") == "income" {
                "売上"
            } else {
                "経費"
            },
        );
        text("lf-review-date", format!("{year}年{month}月{day}日"));
        text("lf-review-category", val("lf-category"));
        text("lf-review-amount", yen(num("lf-amount")));
    }
    if guide_dialog::current(a) == Some(guide_dialog::Page::Ledger) {
        el("guide-dialog-body").set_scroll_top(0);
    }
    focus_without_scroll(&el(&format!("ledger-heading-{}", step.id())))
}

pub(super) fn back(a: &mut App) -> Result<(), JsValue> {
    if let Some(previous) = a.ledger_step.index().checked_sub(1) {
        if !a.storage_protected {
            hidden("lf-submit-feedback", true);
        }
        display(a, Step::ALL[previous])?;
    }
    Ok(())
}

pub(super) fn advance(a: &mut App) -> Result<(), JsValue> {
    if a.demo.is_some() && guide_dialog::current(a) == Some(guide_dialog::Page::Ledger) {
        return Ok(());
    }
    if a.storage_protected {
        feedback(
            "保存済みデータの読み込みに失敗したため、上書きを停止しています。",
            false,
        );
        return Ok(());
    }
    let step = a.ledger_step;
    let (year, month, day) = date();
    if step == Step::Date && !ledger::valid_date(year, month, day) {
        feedback("記帳する日付を確認してください。", false);
        return focus_without_scroll(&el("lf-year"));
    }
    if step == Step::Amount && !(1..=1_000_000_000).contains(&num("lf-amount")) {
        feedback("金額は1〜10億円の整数で入力してください。", false);
        return focus_without_scroll(&el("lf-amount"));
    }
    hidden("lf-submit-feedback", true);
    if step == Step::Review {
        return add_entry(a);
    }
    display(a, Step::ALL[step.index() + 1])
}

pub(super) fn restart(a: &mut App) -> Result<(), JsValue> {
    display(a, Step::Kind)
}
