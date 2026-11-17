use js_sys::Date;
use wasm_bindgen::JsValue;

const DAY_MS: f64 = 86_400_000.0;

pub fn today_in_japan() -> Date {
    let now = Date::new(&JsValue::from_f64(Date::now() + DAY_MS * 9.0 / 24.0));
    date(
        now.get_utc_full_year(),
        now.get_utc_month(),
        now.get_utc_date(),
    )
}

// UTC is used only for calendar arithmetic, after determining the Japanese date.
fn date(year: u32, month: u32, day: u32) -> Date {
    let result = Date::new(&JsValue::from_f64(0.0));
    result.set_utc_full_year_with_month_date(year, month as i32, day as i32);
    result
}

pub fn iso_date(value: &Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        value.get_utc_full_year(),
        value.get_utc_month() + 1,
        value.get_utc_date(),
    )
}

pub fn japanese_date(value: &Date) -> String {
    format!(
        "{}年{}月{}日",
        value.get_utc_full_year(),
        value.get_utc_month() + 1,
        value.get_utc_date(),
    )
}

pub fn parse_date(value: &str) -> Option<Date> {
    let parsed = Date::new(&JsValue::from_str(value));
    (parsed.get_time().is_finite() && iso_date(&parsed) == value).then_some(parsed)
}

fn regular_deadline(year: u32) -> Date {
    let deadline = date(year, 2, 15);
    let offset = match deadline.get_utc_day() {
        6 => 2,
        0 => 1,
        _ => 0,
    };
    date(year, 2, 15 + offset)
}

fn new_business_deadline(opened: &Date) -> Option<Date> {
    let year = opened.get_utc_full_year();
    let target_month = opened.get_utc_month() + 2;
    let last_day = date(year, target_month + 1, 0).get_utc_date();
    let deadline = date(year, target_month, opened.get_utc_date().min(last_day));
    loop {
        // Cabinet Office's published calendar. Do not guess unannounced holidays.
        let holidays: &[u32] = match deadline.get_utc_full_year() {
            2026 => &[
                101, 112, 211, 223, 320, 429, 503, 504, 505, 506, 720, 811, 921, 922, 923, 1012,
                1103, 1123,
            ],
            2027 => &[
                101, 111, 211, 223, 321, 322, 429, 503, 504, 505, 719, 811, 920, 923, 1011, 1103,
                1123,
            ],
            _ => return None,
        };
        let month_day = (deadline.get_utc_month() + 1) * 100 + deadline.get_utc_date();
        let closed = matches!(deadline.get_utc_day(), 0 | 6)
            || matches!(month_day, 102 | 103 | 1229..=1231)
            || holidays.contains(&month_day);
        if !closed {
            return Some(deadline);
        }
        deadline.set_utc_date(deadline.get_utc_date() + 1);
    }
}

// Returns the earliest tax year and its deadline under the ordinary application rules.
pub fn earliest_year(today: &Date, opened: Option<&Date>) -> Option<(u32, Date)> {
    let year = today.get_utc_full_year();
    if let Some(opened) = opened {
        let opened_year = opened.get_utc_full_year();
        let opened_this_year = opened_year == year;
        // An October or later opening can have a deadline that crosses New Year's.
        let previous_year_deadline =
            opened_year + 1 == year && opened.get_utc_month() >= 9 && today.get_utc_month() <= 2;
        if (opened_this_year || previous_year_deadline)
            && opened.get_time() >= date(opened_year, 0, 16).get_time()
        {
            let deadline = new_business_deadline(opened)?;
            if today.get_time() <= deadline.get_time() {
                return Some((opened_year, deadline));
            }
        }
    }
    let deadline = regular_deadline(year);
    if today.get_time() <= deadline.get_time() {
        Some((year, deadline))
    } else {
        Some((year + 1, regular_deadline(year + 1)))
    }
}
