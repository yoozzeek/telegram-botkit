use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn back_button(text: &str, data: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(text, data)]])
}

pub fn rows(rows: Vec<Vec<(&str, &str)>>) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(
        rows.into_iter()
            .map(|r| {
                r.into_iter()
                    .map(|(t, d)| InlineKeyboardButton::callback(t, d))
                    .collect::<Vec<InlineKeyboardButton>>()
            })
            .collect::<Vec<Vec<InlineKeyboardButton>>>(),
    )
}

pub fn to_row(items: Vec<(String, String)>) -> Vec<InlineKeyboardButton> {
    items
        .into_iter()
        .map(|(t, d)| InlineKeyboardButton::callback(t, d))
        .collect()
}

pub fn label_selected(base: &str, selected: bool, selected_prefix: &str) -> String {
    if selected {
        format!("{selected_prefix} {base}")
    } else {
        base.to_string()
    }
}

pub fn toggle_label(base: &str, on: bool, on_icon: Option<&str>, off_icon: Option<&str>) -> String {
    match (on, on_icon, off_icon) {
        (true, Some(icon), _) => format!("{icon} {base}"),
        (false, _, Some(icon)) => format!("{icon} {base}"),
        _ => base.to_string(),
    }
}

pub fn selected_label_with(
    base: &str,
    selected: bool,
    selected_icon: Option<&str>,
    unselected_icon: Option<&str>,
) -> String {
    if selected {
        match selected_icon {
            Some(icon) => format!("{icon} {base}"),
            None => base.to_string(),
        }
    } else {
        match unselected_icon {
            Some(icon) => format!("{icon} {base}"),
            None => base.to_string(),
        }
    }
}

pub fn toggle_icon(on: bool, on_icon: Option<&str>, off_icon: Option<&str>) -> String {
    if on {
        on_icon.unwrap_or("").to_string()
    } else {
        off_icon.unwrap_or("").to_string()
    }
}

pub fn toggles_row(
    items: Vec<(String, String, bool)>,
    on_icon: Option<&str>,
    off_icon: Option<&str>,
) -> Vec<InlineKeyboardButton> {
    items
        .into_iter()
        .map(|(base, data, on)| {
            let label = toggle_label(&base, on, on_icon, off_icon);
            InlineKeyboardButton::callback(label, data)
        })
        .collect()
}

pub fn choice_row(
    items: Vec<(String, String, bool)>,
    selected_icon: Option<&str>,
    unselected_icon: Option<&str>,
) -> Vec<InlineKeyboardButton> {
    items
        .into_iter()
        .map(|(base, data, selected)| {
            let label = selected_label_with(&base, selected, selected_icon, unselected_icon);
            InlineKeyboardButton::callback(label, data)
        })
        .collect()
}
