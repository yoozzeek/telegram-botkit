use crate::session::{UiDialogueStorage, UiStore};
use crate::ui::prelude::UiRequester;

use dispatching::dialogue::Dialogue;
use std::time::Duration;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::types::{ChatId, InlineKeyboardMarkup, LinkPreviewOptions, MessageId, ParseMode};
use teloxide::{dispatching, requests};

#[derive(Default, Clone)]
pub struct ReplyOptions {
    pub markup: Option<InlineKeyboardMarkup>,
    pub parse_mode: Option<ParseMode>,
    pub disable_web_page_preview: Option<bool>,
}

#[derive(Default, Clone)]
pub struct EditOptions {
    pub markup: Option<InlineKeyboardMarkup>,
    pub parse_mode: Option<ParseMode>,
    pub disable_web_page_preview: Option<bool>,
}

pub async fn compact_reply<R, D, S>(
    bot: &R,
    chat: ChatId,
    d: &Dialogue<D, S>,
    previous: Option<MessageId>,
    text: impl Into<String>,
    opts: ReplyOptions,
    ttl: Option<Duration>,
) -> Result<MessageId, teloxide::RequestError>
where
    R: UiRequester,
    <R as requests::Requester>::SendMessage: Send,
    <R as requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    if let Some(prev) = previous {
        if let Err(e) = bot.delete_message(chat, prev).await {
            tracing::debug!(
                error=?e,
                chat=%chat.0,
                prev=%prev.0,
                "delete previous message failed",
            );
        }
    }

    let mut req = bot.send_message(chat, text.into());

    if let Some(pm) = opts.parse_mode {
        req = req.parse_mode(pm);
    }

    if let Some(markup) = opts.markup.clone() {
        req = req.reply_markup(markup);
    }

    if let Some(disable) = opts.disable_web_page_preview {
        if disable {
            req = req.link_preview_options(LinkPreviewOptions {
                is_disabled: true,
                url: None,
                prefer_small_media: false,
                prefer_large_media: false,
                show_above_text: false,
            });
        }
    }

    let msg = req.await?;
    let new_id = msg.id;

    if let Some(dur) = ttl {
        let bot2 = bot.clone();
        tokio::spawn(async move {
            tokio::time::sleep(dur).await;

            if let Err(e) = bot2.delete_message(chat, new_id).await {
                tracing::debug!(
                    error=?e,
                    chat=%chat.0,
                    id=%new_id.0,
                    "ephemeral TTL delete failed",
                );
            }
        });
    }

    // remember in dialogue state
    if let Ok(mut s) = d.get_or_default().await {
        let cur_scene_json = s.ui_get_current_scene_json();

        s.ui_set_scene_for_message(new_id.0, cur_scene_json);
        s.ui_set_last_action_message_id(Some(new_id.0));

        if let Err(e) = d.update(s).await {
            tracing::error!(
                error=?e,
                chat=%chat.0,
                mid=%new_id.0,
                "dialogue update failed (compact_reply)",
            );
        }
    }

    Ok(new_id)
}

pub async fn refresh_or_reply_with<R, D, S>(
    bot: &R,
    chat: ChatId,
    d: &Dialogue<D, S>,
    text: impl Into<String>,
    opts: EditOptions,
) -> Result<MessageId, teloxide::RequestError>
where
    R: UiRequester,
    <R as requests::Requester>::EditMessageText: Send,
    <R as requests::Requester>::SendMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    let mut to_mid: Option<MessageId> = None;

    if let Ok(s) = d.get_or_default().await {
        if let Some(last) = s.ui_get_last_action_message_id() {
            to_mid = Some(MessageId(last));
        }
    }

    let text_owned: String = text.into();

    if let Some(mid) = to_mid {
        // try edit existing
        let mut req = bot.edit_message_text(chat, mid, text_owned.clone());
        if let Some(mk) = opts.markup.clone() {
            req = req.reply_markup(mk);
        }

        if let Some(pm) = opts.parse_mode {
            req = req.parse_mode(pm);
        }

        if let Some(disable) = opts.disable_web_page_preview {
            if disable {
                req = req.link_preview_options(LinkPreviewOptions {
                    is_disabled: true,
                    url: None,
                    prefer_small_media: false,
                    prefer_large_media: false,
                    show_above_text: false,
                });
            }
        }

        match req.await {
            Ok(msg) => return Ok(msg.id),
            Err(e) => {
                // fallback to reply new below
                tracing::debug!(
                    error=?e,
                    chat=%chat.0,
                    last_mid=?to_mid.map(|m| m.0),
                    "edit failed, falling back to send new",
                );
            }
        }
    }

    // reply new
    let mut req = bot.send_message(chat, text_owned);

    if let Some(mk) = opts.markup.clone() {
        req = req.reply_markup(mk);
    }

    if let Some(pm) = opts.parse_mode {
        req = req.parse_mode(pm);
    }

    if let Some(disable) = opts.disable_web_page_preview {
        if disable {
            req = req.link_preview_options(LinkPreviewOptions {
                is_disabled: true,
                url: None,
                prefer_small_media: false,
                prefer_large_media: false,
                show_above_text: false,
            });
        }
    }

    let msg = req.await?;
    let mid = msg.id;

    if let Ok(mut s) = d.get_or_default().await {
        s.ui_set_last_action_message_id(Some(mid.0));

        if let Err(e) = d.update(s).await {
            tracing::error!(
                error=?e,
                chat=%chat.0,
                mid=%mid.0,
                "dialogue update failed (refresh_or_reply_with)",
            );
        }
    }

    Ok(mid)
}

pub async fn notify_ephemeral<R>(
    bot: &R,
    chat: ChatId,
    text: impl Into<String>,
    ttl: Duration,
) -> Result<MessageId, teloxide::RequestError>
where
    R: UiRequester,
    <R as requests::Requester>::SendMessage: Send,
    <R as requests::Requester>::DeleteMessage: Send,
{
    let msg = bot.send_message(chat, text.into()).await?;
    let id = msg.id;
    let bot2 = bot.clone();

    tokio::spawn(async move {
        tokio::time::sleep(ttl).await;
        if let Err(e) = bot2.delete_message(chat, id).await {
            tracing::debug!(
                error=?e,
                chat=%chat.0,
                mid=%id.0,
                "ephemeral TTL delete failed",
            );
        }
    });

    Ok(id)
}

pub async fn clear_input_prompt_message<R, D, S>(bot: &R, chat: ChatId, d: &Dialogue<D, S>)
where
    R: UiRequester,
    <R as requests::Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: UiDialogueStorage<D>,
    <S as dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    if let Ok(mut s) = d.get_or_default().await {
        if let Some(mid) = s.ui_get_input_prompt_message_id() {
            if let Err(e) = bot.delete_message(chat, MessageId(mid)).await {
                tracing::debug!(
                    error=?e,
                    chat=%chat.0,
                    mid=%mid,
                    "delete prompt failed",
                );
            }

            s.ui_set_input_prompt_message_id(None);

            if let Err(e) = d.update(s).await {
                tracing::error!(
                    error=?e,
                    chat=%chat.0,
                    "dialogue update failed (clear_input_prompt_message)",
                );
            }
        }
    }
}

pub async fn clear_input_prompt_message_id<D, S>(d: &Dialogue<D, S>)
where
    D: UiStore,
    S: dispatching::dialogue::Storage<D>,
    <S as dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug,
{
    if let Ok(mut s) = d.get_or_default().await {
        s.ui_set_input_prompt_message_id(None);

        if let Err(e) = d.update(s).await {
            tracing::error!(
                error=?e,
                "dialogue update failed (clear_input_prompt_message_id)",
            );
        }
    }
}

pub async fn delete_incoming<R: UiRequester>(bot: &R, msg: &teloxide::types::Message) -> bool {
    match bot.delete_message(msg.chat.id, msg.id).await {
        Ok(_) => true,
        Err(e) => {
            tracing::debug!(
                error=?e,
                chat=%msg.chat.id.0,
                mid=%msg.id,
                "delete incoming failed",
            );

            false
        }
    }
}

pub fn sanitize_markdown_v2(s: impl AsRef<str>) -> String {
    let mut out = String::with_capacity(s.as_ref().len() + 8);
    for ch in s.as_ref().chars() {
        match ch {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|'
            | '{' | '}' | '.' | '!' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }

    out
}
