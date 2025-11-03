use crate::ui::prelude::UiRequester;
use teloxide::payloads::AnswerCallbackQuerySetters;
use teloxide::prelude::CallbackQuery;

pub const BACK: &str = "ui:back";
pub const CANCEL: &str = "ui:cancel";
pub const HIDE: &str = "ui:hide";
pub const DISABLE_INFO_NOTIFICATIONS: &str = "ui:disable_info_notifications";

pub async fn answer_callback_safe<R: UiRequester>(bot: &R, q: &CallbackQuery) {
    let _ = bot.answer_callback_query(q.id.clone()).await;
}

pub async fn show_success_alert<R: UiRequester>(
    bot: &R,
    q: &CallbackQuery,
    text: impl Into<String>,
) {
    let _ = bot
        .answer_callback_query(q.id.clone())
        .text(format!("✅ {}", text.into()))
        .show_alert(true)
        .await;
}

pub async fn show_warning_alert<R: UiRequester>(
    bot: &R,
    q: &CallbackQuery,
    text: impl Into<String>,
) {
    let _ = bot
        .answer_callback_query(q.id.clone())
        .text(format!("⚠️ {}", text.into()))
        .show_alert(true)
        .await;
}

pub async fn show_error_alert<R: UiRequester>(bot: &R, q: &CallbackQuery, text: impl Into<String>) {
    let _ = bot
        .answer_callback_query(q.id.clone())
        .text(format!("❌ {}", text.into()))
        .show_alert(true)
        .await;
}
