use crate::{el, esc, focus_without_scroll, hidden, html, status, text};
use rust_tax_engine::model::Action;
use std::cell::RefCell;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Element, HtmlDialogElement};

thread_local! {
    static ACTIONS: RefCell<Vec<Action>> = const { RefCell::new(Vec::new()) };
    static ORIGIN: RefCell<Option<Origin>> = const { RefCell::new(None) };
}

struct Origin {
    trigger: Element,
    x: f64,
    y: f64,
}

pub fn render(actions: &[Action]) {
    ACTIONS.with(|previous| {
        let mut previous = previous.borrow_mut();
        let same_titles = previous.len() == actions.len()
            && previous.iter().zip(actions).all(|(old, new)| {
                old.title == new.title && old.icon == new.icon && old.priority == new.priority
            });
        if !same_titles {
            html(
                "action-list",
                &actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| {
                        format!(
                            "<li class='action-item priority-{}'><button type='button' class='action-trigger' data-action-index='{index}' aria-haspopup='dialog'><span class='action-icon' aria-hidden='true'>{}</span><span class='action-title'>{}</span></button></li>",
                            status(action.priority),
                            esc(&action.icon),
                            esc(&action.title),
                        )
                    })
                    .collect::<String>(),
            );
        }
        previous.clear();
        previous.extend_from_slice(actions);
    });
}

fn dialog() -> HtmlDialogElement {
    el("action-dialog").unchecked_into()
}

fn open(index: usize, trigger: &Element) -> Result<(), JsValue> {
    let Some(action) = ACTIONS.with(|actions| actions.borrow().get(index).cloned()) else {
        return Ok(());
    };
    let dialog = dialog();
    if dialog.open() {
        return Ok(());
    }
    let window = web_sys::window().unwrap();
    let origin = Origin {
        trigger: trigger.clone(),
        x: window.scroll_x()?,
        y: window.scroll_y()?,
    };
    dialog.set_attribute("data-action-priority", status(action.priority))?;
    text("action-dialog-title", action.title);
    text("action-dialog-detail", action.detail);
    hidden("action-dialog-timing", action.when.is_empty());
    text("action-dialog-when", action.when);
    dialog.show_modal()?;
    ORIGIN.with(|saved| saved.replace(Some(origin)));
    el("action-dialog-body").set_scroll_top(0);
    focus_without_scroll(&el("action-dialog-title"))
}

fn restore_origin() -> Result<(), JsValue> {
    let Some(origin) = ORIGIN.with(|saved| saved.take()) else {
        return Ok(());
    };
    if origin.trigger.is_connected() {
        focus_without_scroll(&origin.trigger)?;
    } else if let Some(summary) = el("drawer-actions").query_selector("summary")? {
        focus_without_scroll(&summary)?;
    }
    let window = web_sys::window().unwrap();
    if (window.scroll_x()? - origin.x).abs() > 1.0 || (window.scroll_y()? - origin.y).abs() > 1.0 {
        window.scroll_to_with_x_and_y(origin.x, origin.y);
    }
    Ok(())
}

pub fn click(button: &Element) -> Option<Result<(), JsValue>> {
    if matches!(
        button.id().as_str(),
        "action-dialog-close" | "action-dialog-back"
    ) {
        dialog().close();
        return Some(restore_origin());
    }
    let index = button.get_attribute("data-action-index")?.parse().ok()?;
    Some(open(index, button))
}

pub fn init() -> Result<(), JsValue> {
    let closed = Closure::<dyn FnMut()>::new(|| {
        if !dialog().open() {
            if let Err(error) = restore_origin() {
                web_sys::console::error_1(&error);
            }
        }
    });
    dialog().add_event_listener_with_callback("close", closed.as_ref().unchecked_ref())?;
    closed.forget();
    Ok(())
}
