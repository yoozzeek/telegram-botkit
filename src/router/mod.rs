pub mod compose;
pub mod core;

use crate::session::{UiDialogueStorage, UiStore};
use crate::ui::callback;
use crate::ui::message::{clear_input_prompt_message, delete_incoming};
use crate::ui::prelude::UiRequester;
use crate::viewport::Viewport;
use crate::viewport::store::Store;

use dialogue::Dialogue;
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue,
    payloads::AnswerCallbackQuerySetters,
    prelude::Requester,
    types::{CallbackQuery, ChatId, Message},
};

pub enum AppEvent<'a> {
    Msg(&'a Message),
    Cb(&'a CallbackQuery),
}

pub trait AppCtx
where
    <Self::Bot as Requester>::SendMessage: Send,
    <Self::Bot as Requester>::EditMessageText: Send,
    <Self::Bot as Requester>::DeleteMessage: Send,
    <Self::Bot as Requester>::AnswerCallbackQuery: Send,
{
    type Bot: UiRequester;

    fn bot(&self) -> &Self::Bot;
    fn chat(&self) -> ChatId;
    fn user_id(&self) -> i64;
    fn username(&self) -> Option<&str> {
        None
    }
}

#[derive(Default)]
pub struct RouterBuilder<R> {
    routes: Option<Arc<R>>,
    clear_noise: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("routes not configured")]
    RoutesNotConfigured,
}

impl<R> RouterBuilder<R> {
    pub fn new() -> Self {
        Self {
            routes: None,
            clear_noise: true,
        }
    }

    pub fn with_routes(mut self, routes: R) -> Self {
        self.routes = Some(Arc::new(routes));
        self
    }

    pub fn with_clear_noise(mut self, on: bool) -> Self {
        self.clear_noise = on;
        self
    }

    pub fn build(self) -> Result<Router<R>, BuildError> {
        Ok(Router {
            routes: self.routes.ok_or(BuildError::RoutesNotConfigured)?,
            clear_noise: self.clear_noise,
        })
    }
}

pub struct Router<R> {
    routes: Arc<R>,
    clear_noise: bool,
}

impl<R> Clone for Router<R> {
    fn clone(&self) -> Self {
        Self {
            routes: Arc::clone(&self.routes),
            clear_noise: self.clear_noise,
        }
    }
}

impl<R> Router<R> {
    #[tracing::instrument(
        name = "router.handle",
        skip(self, ctx, vp, d, ev),
        fields(
            chat_id = %ctx.chat().0,
            user_id = %ctx.user_id(),
            username = tracing::field::Empty
        )
    )]
    pub async fn handle<C, D, S, M>(
        &self,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, S>,
        ev: AppEvent<'_>,
    ) -> anyhow::Result<()>
    where
        C: AppCtx + Send + Sync,
        D: UiStore + Send + Sync,
        S: UiDialogueStorage<D>,
        M: Store + Send + Sync,
        R: compose::RouterDispatch<C, D, S, M> + compose::SceneLookup,
        <S as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    {
        if let Some(u) = ctx.username() {
            tracing::Span::current().record("username", u);
        }

        #[cfg(feature = "metrics")]
        {
            let kind: &'static str = match ev {
                AppEvent::Msg(_) => "msg",
                AppEvent::Cb(_) => "cb",
            };
            crate::metrics::router_handle(kind, ctx.chat().0, ctx.user_id());
        }

        match ev {
            AppEvent::Msg(m) => {
                // Commands
                if let Some(text) = m.text() {
                    if text.trim_start().starts_with("/start") {
                        // TODO: app chooses behaviour.
                    }
                }

                // Route to active scene if any
                let mut handled = false;
                if let Ok(Some(s)) = d.get().await {
                    if let Some(active) = s.ui_get_active_scene_id() {
                        handled = self
                            .routes
                            .handle_msg(Some(active.as_str()), ctx, vp, d, m)
                            .await?;
                    } else if let Some(last) = s.ui_get_last_action_message_id() {
                        if let Some(json) = s.ui_get_scene_for_message(last) {
                            if let Ok(id) = serde_json::from_str::<String>(&json) {
                                handled = self
                                    .routes
                                    .handle_msg(Some(id.as_str()), ctx, vp, d, m)
                                    .await?;
                            }
                        }
                    }
                }

                if handled {
                    return Ok(());
                }

                if self.clear_noise {
                    let _deleted = delete_incoming(ctx.bot(), m).await;
                }
            }
            AppEvent::Cb(q) => {
                vp.activate_from_callback(d, q, self.routes.as_ref()).await;

                // Validate callback payload
                // size/charset to reduce abuse.
                if let Some(data) = q.data.as_deref() {
                    if data.len() > 64 || !data.is_ascii() {
                        if let Err(e) = ctx.bot().answer_callback_query(q.id.clone()).await {
                            tracing::warn!(error=?e, "answer_callback_query failed (invalid callback)");
                        }

                        return Ok(());
                    }
                }

                // UI actions first
                if let Some(data) = q.data.as_deref() {
                    if data == callback::CANCEL {
                        clear_input_prompt_message(ctx.bot(), ctx.chat(), d).await;

                        if let Some(msg) = &q.message {
                            if let Err(e) = ctx.bot().delete_message(msg.chat().id, msg.id()).await
                            {
                                tracing::warn!(
                                    error=?e,
                                    chat=%msg.chat().id.0,
                                    mid=%msg.id().0,
                                    "delete message failed (CANCEL)"
                                );
                            }
                        }

                        if let Err(e) = ctx.bot().answer_callback_query(q.id.clone()).await {
                            tracing::warn!(error=?e, "answer_callback_query failed (CANCEL)");
                        }

                        return Ok(());
                    }

                    if data == callback::HIDE {
                        if let Err(e) = ctx.bot().answer_callback_query(q.id.clone()).await {
                            tracing::warn!(error=?e, "answer_callback_query failed (HIDE)");
                        }

                        if let Some(msg) = &q.message {
                            if let Err(e) = ctx.bot().delete_message(msg.chat().id, msg.id()).await
                            {
                                tracing::warn!(
                                    error=?e,
                                    chat=%msg.chat().id.0,
                                    mid=%msg.id().0,
                                    "delete message failed (HIDE)"
                                );
                            }
                        }

                        return Ok(());
                    }

                    if data == callback::DISABLE_NOTIFICATIONS {
                        if let Err(e) = ctx.bot().answer_callback_query(q.id.clone()).await {
                            tracing::warn!(error=?e, "answer_callback_query failed (DISABLE_NOTIFICATIONS)");
                        }

                        return Ok(());
                    }
                }

                // Declarative callback routing via routes
                if self.routes.handle_cb(ctx, vp, d, q).await? {
                    return Ok(());
                }

                // Unknown fallback
                if let Err(e) = ctx
                    .bot()
                    .answer_callback_query(q.id.clone())
                    .text("This menu is no longer active, enter /start command and open this section again.")
                    .show_alert(true)
                    .await {
                    tracing::warn!(error=?e, "answer_callback_query failed (unknown fallback)");
                }
            }
        }

        Ok(())
    }
}
