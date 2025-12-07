#[cfg(feature = "redis")]
pub mod redis;
pub mod store;

use crate::router::compose::SceneLookup;
use crate::scene::RenderPolicy;
use crate::session::{UiDialogueStorage, UiStore};
use crate::ui::message;
use crate::ui::prelude::UiRequester;

use crate::router::core::DIALOGUE_SNAPSHOT_TAG;
use dialogue::Dialogue;
use store::Store;
use teloxide::{
    dispatching::dialogue,
    payloads::{EditMessageTextSetters, SendMessageSetters},
    prelude::{CallbackQuery, Requester},
    sugar::request::RequestReplyExt,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, LinkPreviewOptions, MessageId},
};
use tracing::instrument;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MessageMeta {
    pub scene_id: String,
    pub scene_version: u16,
    pub state_json: Option<String>,
    pub state_ref: Option<String>,
    pub state_checksum: Option<String>,
    pub created_at: i64,
    pub ttl_secs: u32,
}

#[derive(Clone, Debug)]
pub struct MetaSpec {
    pub scene_id: &'static str,
    pub scene_version: u16,
    pub state_json: Option<String>,
    pub state_ref: Option<String>,
    pub ttl_secs: u32,
}

#[derive(Clone)]
pub struct Viewport<M: Store> {
    meta: M,
}

pub const SNAP_TTL_SECS: u32 = 3 * 24 * 60 * 60;

impl<M: Store> Viewport<M> {
    pub fn new(meta: M) -> Self {
        Self { meta }
    }

    #[instrument(name = "viewport.load_meta", skip(self))]
    pub async fn load_meta(&self, chat: ChatId, mid: i32) -> anyhow::Result<Option<MessageMeta>> {
        self.meta.load(chat, mid).await
    }

    #[instrument(name = "viewport.apply_view", skip(self, bot, d, view), fields(chat_id = %chat.0))]
    pub async fn apply_view<R, D, S>(
        &self,
        bot: &R,
        chat: ChatId,
        d: &Dialogue<D, S>,
        view: &crate::scene::View,
        policy: RenderPolicy,
        meta: Option<MetaSpec>,
    ) -> anyhow::Result<()>
    where
        R: UiRequester,
        <R as Requester>::SendMessage: Send,
        <R as Requester>::EditMessageText: Send,
        <R as Requester>::DeleteMessage: Send,
        D: UiStore + Send + Sync,
        S: UiDialogueStorage<D>,
        <S as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    {
        let mut mid_opt: Option<MessageId> = None;

        #[cfg(feature = "metrics")]
        {
            let pol: &'static str = match policy {
                RenderPolicy::EditOrReply => "edit_or_reply",
                RenderPolicy::EditOnly => "edit_only",
                RenderPolicy::SendNew => "send_new",
            };
            crate::metrics::apply_view(pol);
        }

        match policy {
            RenderPolicy::EditOrReply => {
                let mid = message::refresh_or_reply_with(
                    bot,
                    chat,
                    d,
                    view.text.clone(),
                    message::EditOptions {
                        markup: view.markup.clone(),
                        parse_mode: view.parse_mode,
                        disable_web_page_preview: view.disable_web_page_preview,
                    },
                )
                .await?;
                mid_opt = Some(mid);
            }
            RenderPolicy::EditOnly => {
                // Edit existing last action
                // message only; never send new.
                if let Ok(s) = d.get_or_default().await {
                    if let Some(last) = s.ui_get_last_action_message_id() {
                        let mut req =
                            bot.edit_message_text(chat, MessageId(last), view.text.clone());
                        if let Some(mk) = view.markup.clone() {
                            req = req.reply_markup(mk);
                        }

                        if let Some(pm) = view.parse_mode {
                            req = req.parse_mode(pm);
                        }

                        if let Some(disable) = view.disable_web_page_preview {
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
                            Ok(_msg) => {
                                mid_opt = Some(MessageId(last));
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error=?e,
                                    chat=%chat.0,
                                    mid=%last,
                                    "edit message failed (EditOnly), no fallback"
                                );
                            }
                        }
                    } else {
                        tracing::debug!(chat=%chat.0, "EditOnly but no last_action_message_id; skipping edit");
                    }
                }
            }
            RenderPolicy::SendNew => {
                // Clear previous prompt and send
                // a new one with Cancel row if needed.
                message::clear_input_prompt_message(bot, chat, d).await;

                // Build extras + Cancel row
                // when markup is empty (prompt).
                let is_prompt = match &view.markup {
                    Some(mk) => mk.inline_keyboard.is_empty(),
                    None => true,
                };

                if is_prompt {
                    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![];
                    if let Some(mk) = &view.markup {
                        for r in mk.inline_keyboard.clone().into_iter() {
                            rows.push(r);
                        }
                    }

                    rows.push(vec![InlineKeyboardButton::callback(
                        "❌ Close",
                        crate::ui::callback::CANCEL,
                    )]);

                    let markup = InlineKeyboardMarkup::new(rows);

                    let mut req = bot.send_message(chat, view.text.clone());

                    if let Some(pm) = view.parse_mode {
                        req = req.parse_mode(pm);
                    }

                    if let Some(disable) = view.disable_web_page_preview {
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

                    req = req.reply_markup(markup);

                    // Reply to last action once if requested
                    if let Ok(s) = d.get_or_default().await {
                        if s.ui_get_reply_to_last_once() {
                            if let Some(to_mid) = s.ui_get_last_action_message_id() {
                                req = req.reply_to(MessageId(to_mid));
                            }
                        }
                    }

                    let msg = req.await?;
                    let new_id = msg.id;

                    if let Ok(mut s) = d.get_or_default().await {
                        s.ui_set_input_prompt_message_id(Some(new_id.0));

                        if s.ui_get_reply_to_last_once() {
                            s.ui_set_reply_to_last_once(false);
                        }

                        if let Err(e) = d.update(s).await {
                            tracing::error!(
                                error=?e,
                                chat=%chat.0,
                                mid=%new_id.0,
                                "dialogue update failed (apply_view:SendNew)",
                            );
                        }
                    }

                    mid_opt = Some(new_id);
                } else {
                    // Menu/info flow: clear any prompt and send full keyboard as-is
                    message::clear_input_prompt_message(bot, chat, d).await;

                    let mid = message::compact_reply(
                        bot,
                        chat,
                        d,
                        None,
                        view.text.clone(),
                        message::ReplyOptions {
                            markup: view.markup.clone(),
                            parse_mode: view.parse_mode,
                            disable_web_page_preview: view.disable_web_page_preview,
                        },
                        None,
                    )
                    .await?;

                    mid_opt = Some(mid);
                }
            }
        }

        // Update dialogue mapping and persist
        // meta only if we have a concrete message id.
        if let Some(spec) = &meta {
            if let (Some(json), Some(mid)) = (&spec.state_json, mid_opt) {
                if let Ok(mut s) = d.get_or_default().await {
                    let checksum = blake3_hex(json.as_bytes());
                    let env = serde_json::json!({
                        "_tgk": DIALOGUE_SNAPSHOT_TAG,
                        "state": json,
                        "checksum": checksum,
                    });

                    s.ui_set_scene_for_message(mid.0, env.to_string());

                    if let Err(e) = d.update(s).await {
                        tracing::error!(
                            error=?e,
                            chat=%chat.0,
                            mid=%mid.0,
                            "dialogue update failed (apply_view:scene_for_message)",
                        );
                    }
                }
            }
        }

        if let (Some(spec), Some(mid)) = (meta, mid_opt) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let checksum = spec.state_json.as_ref().map(|s| blake3_hex(s.as_bytes()));
            let meta = MessageMeta {
                scene_id: spec.scene_id.to_string(),
                scene_version: spec.scene_version,
                state_json: spec.state_json,
                state_ref: spec.state_ref,
                state_checksum: checksum,
                created_at: now,
                ttl_secs: spec.ttl_secs,
            };

            if let Err(e) = self.meta.save(chat, mid.0, meta).await {
                tracing::error!(
                    error=?e,
                    chat=%chat.0,
                    mid=%mid.0,
                    "meta save failed (apply_view)",
                );
            }
        }

        Ok(())
    }

    #[instrument(name = "viewport.activate_from_callback", skip(self, d, q, lookup))]
    pub async fn activate_from_callback<D, S>(
        &self,
        d: &Dialogue<D, S>,
        q: &CallbackQuery,
        lookup: &dyn SceneLookup,
    ) where
        D: UiStore,
        S: dialogue::Storage<D>,
        <S as dialogue::Storage<D>>::Error: std::fmt::Debug,
    {
        let Some(msg) = q.message.as_ref() else {
            return;
        };
        let mid = msg.id().0;
        let chat = msg.chat().id;
        let data = q.data.as_deref().unwrap_or("");

        if let Ok(mut s) = d.get_or_default().await {
            let is_prompt_click = s.ui_get_input_prompt_message_id() == Some(mid);
            let need_switch = !is_prompt_click && s.ui_get_last_action_message_id() != Some(mid);

            if need_switch {
                s.ui_set_last_action_message_id(Some(mid));
                s.ui_set_reply_to_last_once(true);
            }

            if let Ok(Some(meta)) = self.load_meta(chat, mid).await {
                s.ui_set_scene_for_message(
                    mid,
                    serde_json::to_string(&meta.scene_id).unwrap_or("null".into()),
                );
                s.ui_set_active_scene_id(Some(meta.scene_id));
            } else if let Some((scene_id, _ver)) = lookup.find_scene_for_callback(data) {
                s.ui_set_scene_for_message(mid, format!("\"{scene_id}\""));
                s.ui_set_active_scene_id(Some(scene_id.to_string()));
            }

            if let Err(e) = d.update(s).await {
                tracing::error!(
                    error=?e,
                    chat=%chat.0,
                    mid=%mid,
                    "dialogue update failed (activate_from_callback)",
                );
            }
        }
    }

    // Test/public utility to store
    // meta without sending messages.
    pub async fn save_meta_public(
        &self,
        chat: ChatId,
        mid: i32,
        spec: MetaSpec,
    ) -> anyhow::Result<()> {
        let checksum = spec.state_json.as_ref().map(|s| blake3_hex(s.as_bytes()));
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let meta = MessageMeta {
            scene_id: spec.scene_id.to_string(),
            scene_version: spec.scene_version,
            state_json: spec.state_json,
            state_ref: spec.state_ref,
            state_checksum: checksum,
            created_at,
            ttl_secs: spec.ttl_secs,
        };

        self.meta.save(chat, mid, meta).await
    }
}

#[cfg(feature = "redis")]
impl Viewport<redis::RedisStore> {
    pub async fn redis(url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            meta: redis::RedisStore::new(url).await?,
        })
    }
}

pub fn blake3_hex(input: &[u8]) -> String {
    let h = blake3::hash(input);
    hex::encode(h.as_bytes())
}
