use crate::ui::prelude::UiRequester;
use teloxide::payloads::AnswerCallbackQuerySetters;
use teloxide::prelude::CallbackQuery;

pub const BACK: &str = "ui:back";
pub const CANCEL: &str = "ui:cancel";
pub const HIDE: &str = "ui:hide";
pub const DISABLE_NOTIFICATIONS: &str = "ui:disable_notifications";

pub async fn answer_callback_safe<R: UiRequester>(bot: &R, q: &CallbackQuery) {
    if let Err(e) = bot.answer_callback_query(q.id.clone()).await {
        tracing::warn!(error=?e, "answer_callback_query failed (safe)");
    }
}

pub async fn show_success_alert<R: UiRequester>(
    bot: &R,
    q: &CallbackQuery,
    text: impl Into<String>,
) {
    if let Err(e) = bot
        .answer_callback_query(q.id.clone())
        .text(format!("✅ {}", text.into()))
        .show_alert(true)
        .await
    {
        tracing::warn!(error=?e, "show_success_alert failed");
    }
}

pub async fn show_warning_alert<R: UiRequester>(
    bot: &R,
    q: &CallbackQuery,
    text: impl Into<String>,
) {
    if let Err(e) = bot
        .answer_callback_query(q.id.clone())
        .text(format!("⚠️ {}", text.into()))
        .show_alert(true)
        .await
    {
        tracing::warn!(error=?e, "show_warning_alert failed");
    }
}

pub async fn show_error_alert<R: UiRequester>(bot: &R, q: &CallbackQuery, text: impl Into<String>) {
    if let Err(e) = bot
        .answer_callback_query(q.id.clone())
        .text(format!("❌ {}", text.into()))
        .show_alert(true)
        .await
    {
        tracing::warn!(error=?e, "show_error_alert failed");
    }
}
