use crate::guidance::Topic;
use crate::{el, hidden, render_blue_deadline, text, App};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Element, FocusOptions, HtmlDialogElement, HtmlElement};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Topic(Topic),
    PriorInput,
    Ledger,
    BlueDeadline,
}

impl Page {
    fn panel(self) -> &'static str {
        match self {
            Self::Topic(topic) => topic.id(),
            Self::PriorInput => "prior-input",
            Self::Ledger => "ledger",
            Self::BlueDeadline => "blue-deadline",
        }
    }

    pub fn category(self) -> (&'static str, &'static str) {
        match self {
            Self::Topic(topic) => topic.category(),
            Self::PriorInput => Topic::PriorReturn.category(),
            Self::Ledger => ("ledger", "記帳"),
            Self::BlueDeadline => Topic::Blue.category(),
        }
    }

    fn title(self) -> String {
        match self {
            Self::Topic(topic) => el(&format!("guide-title-{}", topic.id()))
                .text_content()
                .unwrap_or_default(),
            Self::PriorInput => "2025年分の入力と申告状況".into(),
            Self::Ledger => "収支を1件記帳する".into(),
            Self::BlueDeadline => "青色申告の適用年".into(),
        }
    }
}

pub struct State {
    pages: Vec<(Page, Element)>,
    x: f64,
    y: f64,
}

pub fn element() -> HtmlDialogElement {
    el("guide-dialog").unchecked_into()
}

fn focus(element: &Element) -> Result<(), JsValue> {
    let options = FocusOptions::new();
    options.set_prevent_scroll(true);
    element
        .unchecked_ref::<HtmlElement>()
        .focus_with_options(&options)
}

fn move_to(id: &str, parent: &str) -> Result<(), JsValue> {
    let child = el(id);
    let parent = el(parent);
    if child.parent_element().as_ref() != Some(&parent) {
        parent.append_child(&child)?;
    }
    Ok(())
}

fn borrow_input(id: &str, home: &str, slot: &str) -> Result<(), JsValue> {
    let home = el(home);
    let height = home.unchecked_ref::<HtmlElement>().offset_height();
    if height > 0 {
        home.set_attribute("style", &format!("min-height: {height}px"))?;
    }
    move_to(id, slot)
}

fn restore_inputs() -> Result<(), JsValue> {
    move_to("prior-year-fields", "prior-year-fields-home")?;
    el("prior-year-fields").remove_attribute("disabled")?;
    move_to("ledger-form", "ledger-form-home")?;
    for home in ["prior-year-fields-home", "ledger-form-home"] {
        el(home).remove_attribute("style")?;
    }
    let deadline = el("blue-application-content");
    let home = el("blue-application-dialog");
    if deadline.parent_element().as_ref() != Some(&home) {
        home.insert_before(&deadline, Some(&el("btn-blue-application-close")))?;
    }
    text(
        "btn-sync-ledger-to-diag",
        "🔄 台帳の合計額で上の税金診断を更新",
    );
    Ok(())
}

pub fn current(app: &App) -> Option<Page> {
    app.guide_dialog
        .as_ref()?
        .pages
        .last()
        .map(|(page, _)| *page)
}

pub fn update_title(app: &App) {
    if let Some(page) = current(app) {
        text("guide-dialog-title", page.title());
    }
}

fn display(app: &App, page: Page, back_label: &str) -> Result<(), JsValue> {
    restore_inputs()?;
    match page {
        Page::PriorInput => borrow_input(
            "prior-year-fields",
            "prior-year-fields-home",
            "guide-slot-prior-input",
        )?,
        Page::Ledger => {
            borrow_input("ledger-form", "ledger-form-home", "guide-slot-ledger")?;
            text(
                "btn-sync-ledger-to-diag",
                "🔄 台帳の合計を試算に反映して閉じる",
            );
        }
        Page::BlueDeadline => {
            move_to("blue-application-content", "guide-slot-blue-deadline")?;
            render_blue_deadline();
        }
        Page::Topic(_) => {}
    }
    for pane in Topic::ALL.map(Page::Topic).into_iter().chain([
        Page::PriorInput,
        Page::Ledger,
        Page::BlueDeadline,
    ]) {
        hidden(&format!("guide-panel-{}", pane.panel()), pane != page);
    }
    let demo = app.demo.is_some();
    for id in ["guide-prior-demo-note", "guide-ledger-demo-note"] {
        hidden(id, !demo);
    }
    for id in ["prior-year-fields", "guide-slot-ledger"] {
        el(id).toggle_attribute_with_force("disabled", demo)?;
    }
    let (category, label) = page.category();
    element().set_attribute("data-guide-category", category)?;
    let location = match page {
        Page::PriorInput => " / 入力",
        Page::Ledger => " / 入力",
        Page::BlueDeadline => " / 期限の確認",
        Page::Topic(_) => "",
    };
    text(
        "guide-dialog-location",
        format!("手続きガイド / {label}{location}"),
    );
    text("guide-dialog-title", page.title());
    text("guide-dialog-back", back_label);
    el("guide-dialog-body").set_scroll_top(0);
    Ok(())
}

pub fn open(app: &mut App, page: Page, trigger: Element) -> Result<(), JsValue> {
    if let Some(state) = &mut app.guide_dialog {
        if state
            .pages
            .last()
            .is_some_and(|(current, _)| *current == page)
        {
            return Ok(());
        }
        state.pages.push((page, trigger));
        display(app, page, "← 案内に戻る")?;
    } else {
        let window = web_sys::window().unwrap();
        let state = State {
            pages: vec![(page, trigger)],
            x: window.scroll_x()?,
            y: window.scroll_y()?,
        };
        display(app, page, root_back_label(&state.pages[0].1))?;
        if let Err(error) = element().show_modal() {
            restore_inputs()?;
            return Err(error);
        }
        app.guide_dialog = Some(state);
    }
    focus(&el("guide-dialog-title"))
}

fn root_back_label(trigger: &Element) -> &'static str {
    if trigger
        .closest("#drawer-next-steps")
        .ok()
        .flatten()
        .is_some()
    {
        "← チェックリストに戻る"
    } else {
        "← 試算に戻る"
    }
}

pub fn back(app: &mut App) -> Result<(), JsValue> {
    let Some(state) = &mut app.guide_dialog else {
        return Ok(());
    };
    if state.pages.len() == 1 {
        return close(app);
    }
    let (_, trigger) = state.pages.pop().unwrap();
    let (parent, parent_trigger) = state.pages.last().unwrap();
    let parent = *parent;
    let label = if state.pages.len() > 1 {
        "← 案内に戻る"
    } else {
        root_back_label(parent_trigger)
    };
    display(app, parent, label)?;
    focus(&trigger)
}

pub fn close(app: &mut App) -> Result<(), JsValue> {
    element().close();
    finish(app)
}

// Also runs after a native keyboard dismissal.
pub fn finish(app: &mut App) -> Result<(), JsValue> {
    let Some(state) = app.guide_dialog.take() else {
        return Ok(());
    };
    restore_inputs()?;
    let origin = &state.pages[0].1;
    let fallback = if el("next-step").class_list().contains("hidden") {
        el("comparison-title")
    } else {
        el("btn-next-step")
    };
    let origin = if origin.closest(".hidden")?.is_some() {
        &fallback
    } else {
        origin
    };
    focus(origin)?;
    let window = web_sys::window().unwrap();
    if (window.scroll_x()? - state.x).abs() > 1.0 || (window.scroll_y()? - state.y).abs() > 1.0 {
        window.scroll_to_with_x_and_y(state.x, state.y);
    }
    Ok(())
}
