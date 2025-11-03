use crate::session::UiStore;
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => match parse_sol_to_lamports(text) {
            Some(v) if range.contains(&v) => {
                apply(v);

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    success_msg(v),
                    std::time::Duration::from_secs(3),
                )
                .await;

                true
            }
            _ => {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => match parse_percent_to_bp(text.trim()) {
            Some(bp) if range.contains(&bp) => {
                apply_bp(bp);

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    success_msg(bp),
                    std::time::Duration::from_secs(3),
                )
                .await;

                true
            }
            _ => {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim();
            if t.starts_with('-') {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
                return false;
            }

            match parse_percent_to_bp(t) {
                Some(bp) if range.contains(&bp) => {
                    apply_bp(bp);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        success_msg(bp),
                        std::time::Duration::from_secs(3),
                    )
                    .await;

                    true
                }
                _ => {
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        err_text,
                        std::time::Duration::from_secs(3),
                    )
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim();
            if t.starts_with('+') {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
                return false;
            }

            match parse_percent_to_bp(t) {
                Some(bp) if range.contains(&bp) => {
                    apply_bp(bp);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        success_msg(bp),
                        std::time::Duration::from_secs(3),
                    )
                    .await;

                    true
                }
                _ => {
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        err_text,
                        std::time::Duration::from_secs(3),
                    )
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => match parse_time_duration(text.trim()) {
            Some(secs) if secs >= min_secs => {
                apply_secs(secs);

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    success_msg(secs),
                    std::time::Duration::from_secs(3),
                )
                .await;

                true
            }
            _ => {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim().replace([',', ' ', '_'], "");
            match t.parse::<u64>() {
                Ok(v) => {
                    apply(v);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        success_msg(v),
                        std::time::Duration::from_secs(3),
                    )
                    .await;

                    true
                }
                Err(_) => {
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        err_text,
                        std::time::Duration::from_secs(3),
                    )
                    .await;
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(u64),
    M: Fn(u64) -> String,
    V: Fn(u64) -> bool,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim().replace([',', ' ', '_'], "");
            match t.parse::<u64>() {
                Ok(v) if validate(v) => {
                    apply(v);

                    clear_input_prompt_message(bot, msg.chat.id, d).await;
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        success_msg(v),
                        std::time::Duration::from_secs(3),
                    )
                    .await;

                    true
                }
                _ => {
                    let _ = notify_ephemeral(
                        bot,
                        msg.chat.id,
                        err_text,
                        std::time::Duration::from_secs(3),
                    )
                    .await;
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(String),
    M: Fn(&str) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => {
            let t = text.trim();
            if t.is_empty() {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
                false
            } else {
                apply(t.to_string());

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    success_msg(t),
                    std::time::Duration::from_secs(3),
                )
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
    R: UiRequester + teloxide::requests::Requester,
    <R as teloxide::requests::Requester>::SendMessage: Send,
    <R as teloxide::requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
    <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    F: FnMut(String),
    M: Fn(&str) -> String,
{
    if d.get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_none()
    {
        let _ = bot.delete_message(msg.chat.id, msg.id).await;
        return false;
    }

    match msg.text() {
        Some(text) => match parse_solana_address(text) {
            Some(addr) => {
                apply(addr.clone());

                clear_input_prompt_message(bot, msg.chat.id, d).await;
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    success_msg(&addr),
                    std::time::Duration::from_secs(3),
                )
                .await;

                true
            }
            None => {
                let _ = notify_ephemeral(
                    bot,
                    msg.chat.id,
                    err_text,
                    std::time::Duration::from_secs(3),
                )
                .await;
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
