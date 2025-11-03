use crate::compose;
use crate::scene::{Ctx as SceneCtx, Effect, MsgPattern, RenderPolicy, Scene, UiEffect};
use crate::session::UiStore;
use crate::ui::message;
use crate::ui::prelude::UiRequester;
use crate::viewport::{MetaSpec, SNAP_TTL_SECS, Viewport, store};

use super::AppCtx;

use dialogue::Dialogue;
use std::future::Future;
use std::pin::Pin;
use teloxide::dispatching::dialogue;
use teloxide::prelude::Requester;
use teloxide::types::{
    CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, MessageId,
    ParseMode,
};
use tracing::instrument;

#[instrument(
    name = "router.ui_effects",
    skip(bot, d, ui),
    fields(chat_id = %chat.0, effects = %ui.len())
)]
pub async fn run_ui_effects<R, D, S>(bot: &R, chat: ChatId, d: &Dialogue<D, S>, ui: &Vec<UiEffect>)
where
    R: UiRequester + teloxide::requests::Requester,
    <R as Requester>::SendMessage: Send,
    <R as Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    S: dialogue::Storage<D> + Send + Sync,
    <S as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    for eff in ui {
        match eff {
            UiEffect::Notification { text_md, ttl_secs } => {
                let kb = InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback(
                        "Disable Notifications",
                        crate::ui::callback::DISABLE_INFO_NOTIFICATIONS,
                    ),
                    InlineKeyboardButton::callback("Hide", crate::ui::callback::HIDE),
                ]]);

                let text = message::sanitize_markdown_v2(text_md);
                let ttl = ttl_secs.map(std::time::Duration::from_secs);

                match message::compact_reply(
                    bot,
                    chat,
                    d,
                    None,
                    text,
                    message::ReplyOptions {
                        markup: Some(kb),
                        parse_mode: Some(ParseMode::MarkdownV2),
                        disable_web_page_preview: Some(false),
                    },
                    ttl,
                )
                .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error=?e, chat=%chat.0, "notification send failed");
                    }
                }
            }
            UiEffect::ClearPrompt => {
                message::clear_input_prompt_message(bot, chat, d).await;
            }
        }
    }
}

#[instrument(
    name = "router.restore_state",
    skip(scene, vp, d, sctx),
    fields(
        scene_id = %S::ID,
        source_mid = source.as_ref().map(|(_, m)| m.0).unwrap_or_default(),
        restore_path = tracing::field::Empty
    )
)]
pub async fn restore_state<S: Scene, D, St, M>(
    scene: &S,
    vp: &Viewport<M>,
    d: &Dialogue<D, St>,
    sctx: &SceneCtx,
    source: Option<(ChatId, MessageId)>,
) -> (S::State, &'static str)
where
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    M: store::Store + Send + Sync,
{
    let mut out_state: Option<S::State> = None;
    let mut label: &'static str = "init";

    if let Some((chat_id, mid)) = source {
        if let Ok(Some(sess)) = d.get().await {
            if sess.ui_get_last_action_message_id() == Some(mid.0) {
                if let Some(json) = sess.ui_get_scene_for_message(mid.0) {
                    let snap = crate::scene::Snapshot {
                        scene_id: S::ID,
                        scene_version: S::VERSION,
                        state_json: Some(json.as_str()),
                        state_checksum: None,
                    };

                    if let Some(st) = scene.restore(snap) {
                        out_state = Some(st);
                        label = "dialogue";
                    }
                }
            }
        }

        if out_state.is_none() {
            if let Ok(Some(meta)) = vp.load_meta(chat_id, mid.0).await {
                let snap = crate::scene::Snapshot {
                    scene_id: &meta.scene_id,
                    scene_version: meta.scene_version,
                    state_json: meta.state_json.as_deref(),
                    state_checksum: meta.state_checksum.as_deref(),
                };

                if let Some(st) = scene.restore(snap) {
                    out_state = Some(st);
                    label = "meta";
                } else {
                    label = "mismatch";
                }
            }
        }
    }

    let state = out_state.unwrap_or_else(|| scene.init(sctx));
    let path_label = label;

    tracing::Span::current().record("restore_path", path_label);

    (state, label)
}

#[instrument(
    name = "router.apply_effect",
    skip(routes, scene, ctx, vp, d, sctx, eff),
    fields(
        scene_id = %S::ID,
        chat_id = %ctx.chat().0,
        user_id = %ctx.user_id(),
        effect = tracing::field::Empty
    )
)]
pub async fn apply_effect<S, C, D, St, M, R>(
    routes: &R,
    scene: &S,
    ctx: &C,
    vp: &Viewport<M>,
    d: &Dialogue<D, St>,
    sctx: &SceneCtx,
    eff: Effect<S::State>,
) -> anyhow::Result<()>
where
    S: Scene,
    C: AppCtx + Sync,
    <C::Bot as Requester>::SendMessage: Send,
    <C::Bot as Requester>::EditMessageText: Send,
    <C::Bot as Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    M: store::Store + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    R: compose::RouteDispatch,
{
    let eff_label: &str = match &eff {
        Effect::Stay(_, _) => "Stay",
        Effect::StayWithEffect(_, _, _) => "StayWithEffect",
        Effect::SwitchScene(_) => "SwitchScene",
        Effect::Noop => "Noop",
        Effect::NoopWithEffect(_) => "NoopWithEffect",
    };
    tracing::Span::current().record("effect", eff_label);

    match eff {
        Effect::Stay(ns, pol) => {
            let view = scene.render(sctx, &ns);
            let snap = scene.snapshot(&ns);

            if let Ok(mut s) = d.get_or_default().await {
                s.ui_set_active_scene_id(Some(S::ID.to_string()));

                if let Err(e) = d.update(s).await {
                    tracing::error!(
                        error=?e,
                        chat=%ctx.chat().0,
                        scene_id=%S::ID,
                        "dialogue update failed (apply_effect:Stay)"
                    );
                }
            }

            vp.apply_view(
                ctx.bot(),
                ctx.chat(),
                d,
                &view,
                pol,
                Some(MetaSpec {
                    scene_id: S::ID,
                    scene_version: S::VERSION,
                    state_json: snap.0,
                    state_ref: snap.1,
                    ttl_secs: SNAP_TTL_SECS,
                }),
            )
            .await?;
        }
        Effect::StayWithEffect(ns, pol, ui) => {
            let view = scene.render(sctx, &ns);
            let snap = scene.snapshot(&ns);

            if let Ok(mut s) = d.get_or_default().await {
                s.ui_set_active_scene_id(Some(S::ID.to_string()));

                if let Err(e) = d.update(s).await {
                    tracing::error!(
                        error=?e,
                        chat=%ctx.chat().0,
                        scene_id=%S::ID,
                        "dialogue update failed (apply_effect:StayWithEffect)"
                    );
                }
            }

            vp.apply_view(
                ctx.bot(),
                ctx.chat(),
                d,
                &view,
                pol,
                Some(MetaSpec {
                    scene_id: S::ID,
                    scene_version: S::VERSION,
                    state_json: snap.0,
                    state_ref: snap.1,
                    ttl_secs: SNAP_TTL_SECS,
                }),
            )
            .await?;

            run_ui_effects(ctx.bot(), ctx.chat(), d, &ui).await;
        }
        Effect::SwitchScene(sw) => {
            let _ = routes
                .switch_to_scene_by_id(sw.to_scene_id, ctx, vp, d)
                .await?;
        }
        Effect::Noop => {}
        Effect::NoopWithEffect(ui) => {
            run_ui_effects(ctx.bot(), ctx.chat(), d, &ui).await;
        }
    }
    Ok(())
}

#[instrument(
    name = "router.init_and_render",
    skip(scene, ctx, vp, d),
    fields(
        scene_id = %S::ID,
        chat_id = %ctx.chat().0,
        user_id = %ctx.user_id()
    )
)]
pub async fn init_and_render<S, C, D, St, M>(
    scene: &S,
    ctx: &C,
    vp: &Viewport<M>,
    d: &Dialogue<D, St>,
) -> anyhow::Result<()>
where
    S: Scene,
    C: AppCtx + Sync,
    <C::Bot as Requester>::SendMessage: Send,
    <C::Bot as Requester>::EditMessageText: Send,
    <C::Bot as Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    M: store::Store + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    let sctx = SceneCtx {
        user_id: ctx.user_id(),
    };
    let state = scene.init(&sctx);
    let view = scene.render(&sctx, &state);
    let snap = scene.snapshot(&state);

    if let Ok(mut s) = d.get_or_default().await {
        s.ui_set_active_scene_id(Some(S::ID.to_string()));

        if let Err(e) = d.update(s).await {
            tracing::error!(
                error=?e,
                chat=%ctx.chat().0,
                scene_id=%S::ID,
                "dialogue update failed (init_and_render)"
            );
        }
    }

    vp.apply_view(
        ctx.bot(),
        ctx.chat(),
        d,
        &view,
        RenderPolicy::EditOrReply,
        Some(MetaSpec {
            scene_id: S::ID,
            scene_version: S::VERSION,
            state_json: snap.0,
            state_ref: snap.1,
            ttl_secs: SNAP_TTL_SECS,
        }),
    )
    .await?;

    Ok(())
}

#[instrument(
    name = "router.run_cb",
    skip(scene, routes, ctx, vp, d, q),
    fields(
        scene_id = %S::ID,
        chat_id = %ctx.chat().0,
        user_id = %ctx.user_id()
    )
)]
pub async fn run_cb<S, C, D, St, M, R>(
    scene: &S,
    routes: &R,
    ctx: &C,
    vp: &Viewport<M>,
    d: &Dialogue<D, St>,
    q: &CallbackQuery,
) -> anyhow::Result<bool>
where
    S: Scene,
    C: AppCtx + Sync,
    <C::Bot as Requester>::AnswerCallbackQuery: Send,
    <C::Bot as Requester>::SendMessage: Send,
    <C::Bot as Requester>::EditMessageText: Send,
    <C::Bot as Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    M: store::Store + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    R: compose::RouteDispatch,
{
    let sctx = SceneCtx {
        user_id: ctx.user_id(),
    };

    if let Some(ev) = scene.bindings().cb.iter().find_map(|b| (b.to_event)(q)) {
        let source = q.message.as_ref().map(|m| (m.chat().id, m.id()));
        let (state, _rpath) = restore_state(scene, vp, d, &sctx, source).await;
        let eff = scene.update(&sctx, &state, ev);

        apply_effect(routes, scene, ctx, vp, d, &sctx, eff).await?;

        if let Err(e) = ctx.bot().answer_callback_query(q.id.clone()).await {
            tracing::warn!(error=?e, chat=%ctx.chat().0, "answer_callback_query failed");
        }

        return Ok(true);
    }

    Ok(false)
}

type EntryFuture<'a, S> = Pin<Box<dyn Future<Output = Option<<S as Scene>::State>> + Send + 'a>>;

pub type EntryHandler<S, C, D, St> = for<'a> fn(
    &'a <C as super::AppCtx>::Bot,
    &'a Dialogue<D, St>,
    &'a Message,
    &'a <S as Scene>::State,
) -> EntryFuture<'a, S>;

#[instrument(
    name = "router.run_msg",
    skip(scene, routes, entry, ctx, vp, d, m),
    fields(
        scene_id = %S::ID,
        chat_id = %ctx.chat().0,
        user_id = %ctx.user_id()
    )
)]
pub async fn run_msg<S, C, D, St, M, R>(
    scene: &S,
    routes: &R,
    entry: Option<EntryHandler<S, C, D, St>>,
    ctx: &C,
    vp: &Viewport<M>,
    d: &Dialogue<D, St>,
    m: &Message,
) -> anyhow::Result<bool>
where
    S: Scene,
    C: AppCtx + Sync,
    <C::Bot as Requester>::SendMessage: Send,
    <C::Bot as Requester>::EditMessageText: Send,
    <C::Bot as Requester>::DeleteMessage: Send,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    M: store::Store + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    R: compose::RouteDispatch,
{
    let sctx = SceneCtx {
        user_id: ctx.user_id(),
    };

    // if prompt active, try entry flow via viewport meta
    if let Ok(Some(s)) = d.get().await {
        if let Some(pid) = s.ui_get_input_prompt_message_id() {
            if let Ok(Some(meta)) = vp.load_meta(ctx.chat(), pid).await {
                if let Some(cur) = scene.restore(crate::scene::Snapshot {
                    scene_id: &meta.scene_id,
                    scene_version: meta.scene_version,
                    state_json: meta.state_json.as_deref(),
                    state_checksum: meta.state_checksum.as_deref(),
                }) {
                    let _deleted = message::delete_incoming(ctx.bot(), m).await;

                    if let Some(handle) = entry {
                        if let Some(ns) = handle(ctx.bot(), d, m, &cur).await {
                            let view = scene.render(&sctx, &ns);
                            let snap = scene.snapshot(&ns);

                            vp.apply_view(
                                ctx.bot(),
                                ctx.chat(),
                                d,
                                &view,
                                RenderPolicy::EditOrReply,
                                Some(MetaSpec {
                                    scene_id: S::ID,
                                    scene_version: S::VERSION,
                                    state_json: snap.0,
                                    state_ref: snap.1,
                                    ttl_secs: SNAP_TTL_SECS,
                                }),
                            )
                            .await?;

                            return Ok(true);
                        }
                    }
                }
            }
        }
    }

    // otherwise try message bindings;
    // gate AnyText by prompt presence.
    let prompt_active = d
        .get()
        .await
        .ok()
        .flatten()
        .and_then(|s| s.ui_get_input_prompt_message_id())
        .is_some();

    if let Some(ev) = scene.bindings().msg.iter().find_map(|b| match b.pattern {
        MsgPattern::AnyText if !prompt_active => None,
        _ => (b.to_event)(m),
    }) {
        let (state, _rpath) = restore_state(scene, vp, d, &sctx, None).await;
        let eff = scene.update(&sctx, &state, ev);

        apply_effect(routes, scene, ctx, vp, d, &sctx, eff).await?;

        return Ok(true);
    }

    Ok(false)
}
