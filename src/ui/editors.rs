use crate::session::{UiDialogueStorage, UiStore};
use crate::ui::formatters::{
    format_sol, parse_percent_to_bp, parse_sol_to_lamports, parse_solana_address,
    parse_time_duration,
};
use crate::ui::message::{clear_input_prompt_message, notify_ephemeral};
use crate::ui::prelude::UiRequester;

use teloxide::dispatching::dialogue::Dialogue;
use teloxide::prelude::Message;

pub async fn edit_lamports<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    range: core::ops::RangeInclusive<u64>,
    mut apply: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_lamports:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => match parse_sol_to_lamports(text) {
            Some(v) if range.contains(&v) => {
                apply(v);

                clear_input_prompt_message(bot, msg.chat.id, d).await;

                notify_ephemeral_safe(bot, msg.chat.id, success_msg(v), "edit_lamports:ok").await;

                true
            }
            _ => {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_lamports:err").await;

                false
            }
        },
        None => false,
    }
}

pub async fn edit_percent<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    range: core::ops::RangeInclusive<u64>,
    mut apply_bp: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_percent:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => match parse_percent_to_bp(text.trim()) {
            Some(bp) if range.contains(&bp) => {
                apply_bp(bp);

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                notify_ephemeral_safe(bot, msg.chat.id, success_msg(bp), "edit_percent:ok").await;

                true
            }
            _ => {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_percent:err").await;

                false
            }
        },
        None => false,
    }
}

pub async fn edit_percent_positive<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    range: core::ops::RangeInclusive<u64>,
    mut apply_bp: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_percent_positive:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim();
            if t.starts_with('-') {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_percent_positive:sign")
                    .await;

                return false;
            }

            match parse_percent_to_bp(t) {
                Some(bp) if range.contains(&bp) => {
                    apply_bp(bp);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;

                    notify_ephemeral_safe(
                        bot,
                        msg.chat.id,
                        success_msg(bp),
                        "edit_percent_positive:ok",
                    )
                    .await;

                    true
                }
                _ => {
                    notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_percent_positive:err")
                        .await;

                    false
                }
            }
        }
        None => false,
    }
}

pub async fn edit_percent_negative<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    range: core::ops::RangeInclusive<u64>,
    mut apply_bp: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_percent_negative:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim();
            if t.starts_with('+') {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_percent_negative:sign")
                    .await;

                return false;
            }

            match parse_percent_to_bp(t) {
                Some(bp) if range.contains(&bp) => {
                    apply_bp(bp);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;
                    notify_ephemeral_safe(
                        bot,
                        msg.chat.id,
                        success_msg(bp),
                        "edit_percent_negative:ok",
                    )
                    .await;

                    true
                }
                _ => {
                    notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_percent_negative:err")
                        .await;

                    false
                }
            }
        }
        None => false,
    }
}

pub async fn edit_time_secs<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    min_secs: u64,
    mut apply_secs: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_time_secs:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => match parse_time_duration(text.trim()) {
            Some(secs) if secs >= min_secs => {
                apply_secs(secs);

                clear_input_prompt_message(bot, msg.chat.id, d).await;

                notify_ephemeral_safe(bot, msg.chat.id, success_msg(secs), "edit_time_secs:ok")
                    .await;

                true
            }
            _ => {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_time_secs:err").await;
                false
            }
        },
        None => false,
    }
}

pub async fn edit_u64<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    mut apply: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_u64:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim().replace([',', ' ', '_'], "");
            match t.parse::<u64>() {
                Ok(v) => {
                    apply(v);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;

                    notify_ephemeral_safe(bot, msg.chat.id, success_msg(v), "edit_u64:ok").await;

                    true
                }
                Err(_) => {
                    notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_u64:err").await;

                    false
                }
            }
        }
        None => false,
    }
}

pub async fn edit_u64_valid<R, D, S, F, M, V>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    validate: V,
    mut apply: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
    V: Fn(u64) -> bool,
{
    if !ensure_prompt_active(bot, d, msg, "edit_u64_valid:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim().replace([',', ' ', '_'], "");
            match t.parse::<u64>() {
                Ok(v) if validate(v) => {
                    apply(v);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;

                    notify_ephemeral_safe(bot, msg.chat.id, success_msg(v), "edit_u64_valid:ok")
                        .await;

                    true
                }
                _ => {
                    notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_u64_valid:err").await;

                    false
                }
            }
        }
        None => false,
    }
}

pub async fn edit_string_nonempty<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    mut apply: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(String),
    M: Fn(&str) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_string_nonempty:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim();
            if t.is_empty() {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_string_nonempty:empty")
                    .await;

                false
            } else {
                apply(t.to_string());

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                notify_ephemeral_safe(bot, msg.chat.id, success_msg(t), "edit_string_nonempty:ok")
                    .await;

                true
            }
        }
        None => false,
    }
}

pub async fn edit_base58_address<R, D, S, F, M>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    mut apply: F,
    success_msg: M,
    err_text: &str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(String),
    M: Fn(&str) -> String,
{
    if !ensure_prompt_active(bot, d, msg, "edit_base58_address:prompt_inactive").await {
        return false;
    }

    match msg.text() {
        Some(text) => match parse_solana_address(text) {
            Some(addr) => {
                apply(addr.clone());

                clear_input_prompt_message(bot, msg.chat.id, d).await;

                notify_ephemeral_safe(
                    bot,
                    msg.chat.id,
                    success_msg(&addr),
                    "edit_base58_address:ok",
                )
                .await;

                true
            }
            None => {
                notify_ephemeral_safe(bot, msg.chat.id, err_text, "edit_base58_address:err").await;

                false
            }
        },
        None => false,
    }
}

pub fn ok_lamports(prefix: &str, v: u64) -> String {
    format!("✅ {prefix} {}", format_sol(v))
}

pub fn ok_percent(prefix: &str, bp: u64) -> String {
    format!("✅ {prefix} {:.2}%", (bp as f64) / 100.0)
}

pub fn ok_time(prefix: &str, secs: u64) -> String {
    format!(
        "✅ {prefix} {}",
        crate::ui::formatters::format_duration_short(secs)
    )
}

async fn ensure_prompt_active<R, D, S>(
    bot: &R,
    d: &Dialogue<D, S>,
    msg: &Message,
    ctx: &'static str,
) -> bool
where
    R: UiRequester,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_some()
    {
        return true;
    }

    if let Err(e) = bot.delete_message(msg.chat.id, msg.id).await {
        tracing::warn!(
            error=?e,
            chat=%msg.chat.id.0,
            mid=%msg.id,
            "delete message failed ({ctx})",
        );
    }

    false
}

async fn notify_ephemeral_safe<R>(
    bot: &R,
    chat: teloxide::types::ChatId,
    text: impl Into<String>,
    ctx: &'static str,
) where
    R: UiRequester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
{
    if let Err(e) = notify_ephemeral(bot, chat, text, std::time::Duration::from_secs(3)).await {
        tracing::warn!(
            error=?e,
            chat=%chat.0,
            "notify_ephemeral failed ({ctx})",
        );
    }
}
