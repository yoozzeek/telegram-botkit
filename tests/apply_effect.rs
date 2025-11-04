use telegram_botkit::router::AppCtx;
use telegram_botkit::router::core::apply_effect;
use telegram_botkit::scene::*;
use telegram_botkit::session::{SimpleSession, UiStore};
use telegram_botkit::viewport::{MetaSpec, SNAP_TTL_SECS, Viewport, store::NoopStore};

use teloxide::Bot;
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use teloxide::types::ChatId;

use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use teloxide::prelude::Message;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

// -------- test HTTP server returning Telegram-shaped responses --------
async fn handle(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().to_string();
    let method = path.split('/').next_back().unwrap_or("");
    let ok = |body: serde_json::Value| {
        Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
            .unwrap()
    };

    // Generic minimal message
    let msg = serde_json::json!({
        "message_id": 100,
        "date": 0,
        "chat": {"id": 1, "type": "private"},
        "text": "ok"
    });

    let resp = match method {
        "sendMessage" | "editMessageText" => ok(serde_json::json!({"ok": true, "result": msg})),
        "deleteMessage" | "answerCallbackQuery" => {
            ok(serde_json::json!({"ok": true, "result": true}))
        }
        _ => ok(serde_json::json!({"ok": true, "result": msg})),
    };

    Ok(resp)
}

async fn start_test_server() -> (SocketAddr, oneshot::Sender<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, mut rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => break,
                res = listener.accept() => {
                    let (stream, _) = res.unwrap();
                    let io = TokioIo::new(stream);
                    let svc = service_fn(handle);
                    let _ = http1::Builder::new().serve_connection(io, svc).await;
                }
            }
        }
    });

    (addr, tx)
}

fn dialogue() -> Dialogue<SimpleSession, InMemStorage<SimpleSession>> {
    let storage: Arc<InMemStorage<SimpleSession>> = InMemStorage::new();
    Dialogue::new(storage, ChatId(1))
}

struct TestAppCtx {
    bot: Bot,
    chat: ChatId,
}

impl AppCtx for TestAppCtx {
    type Bot = Bot;

    fn bot(&self) -> &Self::Bot {
        &self.bot
    }

    fn chat(&self) -> ChatId {
        self.chat
    }

    fn user_id(&self) -> i64 {
        self.chat.0
    }
}

struct TestScene;

// Dummy message-entry handler
// used by compose builder (monomorphic)
async fn dummy_msg_entry(
    _bot: &Bot,
    _d: &Dialogue<SimpleSession, InMemStorage<SimpleSession>>,
    _m: &Message,
    _cur: &State,
) -> Option<State> {
    None
}

impl Default for TestScene {
    fn default() -> Self {
        TestScene
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum State {
    Root,
}

#[derive(Clone, Debug)]
enum Event {}

impl Scene for TestScene {
    const VERSION: u16 = 1;
    const ID: &'static str = "test_scene";
    const PREFIX: &'static str = "ts";

    type State = State;
    type Event = Event;

    fn init(&self, _c: &Ctx) -> State {
        State::Root
    }

    fn render(&self, _c: &Ctx, _s: &State) -> View {
        View {
            text: "hello".into(),
            markup: None,
            parse_mode: None,
            disable_web_page_preview: None,
        }
    }

    fn update(&self, _c: &Ctx, s: &State, _e: Event) -> Effect<State> {
        Effect::Stay(s.clone(), RenderPolicy::EditOrReply)
    }

    fn bindings(&self) -> Bindings<Event> {
        Bindings {
            msg: vec![],
            cb: vec![],
        }
    }
}

#[tokio::test]
async fn apply_effect_stay_edit_or_reply_sets_last_message() {
    let (addr, shutdown) = start_test_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap());
    let vp = Viewport::new(NoopStore);
    let d = dialogue();
    let ctx = TestAppCtx {
        bot,
        chat: ChatId(1),
    };

    let routes = telegram_botkit::router::compose::Builder::<
        TestAppCtx,
        SimpleSession,
        InMemStorage<SimpleSession>,
        NoopStore,
    >::new()
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene>()
        .msg_entry(|b, d, m, s| Box::pin(dummy_msg_entry(b, d, m, s))),
    )
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene2>()
        .msg_entry(|b, d, m, s| Box::pin(dummy_msg_entry(b, d, m, s))),
    )
    .build()
    .unwrap();

    let sctx = Ctx {
        user_id: ctx.user_id(),
    };
    let eff = Effect::Stay(State::Root, RenderPolicy::EditOrReply);

    apply_effect(&routes, &TestScene, &ctx, &vp, &d, &sctx, eff)
        .await
        .unwrap();

    let st = d.get_or_default().await.unwrap();
    assert!(st.ui_get_last_action_message_id().is_some());

    let _ = shutdown.send(());
}

struct TestScene2;

impl Default for TestScene2 {
    fn default() -> Self {
        TestScene2
    }
}

impl Scene for TestScene2 {
    const VERSION: u16 = 1;
    const ID: &'static str = "test_scene2";
    const PREFIX: &'static str = "t2";

    type State = State;
    type Event = Event;

    fn init(&self, _c: &Ctx) -> State {
        State::Root
    }

    fn render(&self, _c: &Ctx, _s: &State) -> View {
        View {
            text: "s2".into(),
            markup: None,
            parse_mode: None,
            disable_web_page_preview: None,
        }
    }

    fn update(&self, _c: &Ctx, s: &State, _e: Event) -> Effect<State> {
        Effect::Stay(s.clone(), RenderPolicy::EditOrReply)
    }

    fn bindings(&self) -> Bindings<Event> {
        Bindings {
            msg: vec![],
            cb: vec![],
        }
    }
}

#[tokio::test]
async fn apply_effect_stay_with_ui_effects_clears_prompt() {
    let (addr, shutdown) = start_test_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap());
    let vp = Viewport::new(NoopStore);
    let d = dialogue();
    let ctx = TestAppCtx {
        bot,
        chat: ChatId(1),
    };

    let routes = telegram_botkit::router::compose::Builder::<
        TestAppCtx,
        SimpleSession,
        InMemStorage<SimpleSession>,
        NoopStore,
    >::new()
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene>()
        .msg_entry(|b, d, m, s| Box::pin(dummy_msg_entry(b, d, m, s))),
    )
    .build()
    .unwrap();

    // preset prompt
    let mut s = d.get_or_default().await.unwrap();
    s.ui_set_input_prompt_message_id(Some(111));
    d.update(s).await.unwrap();

    let sctx = Ctx {
        user_id: ctx.user_id(),
    };
    let eff = Effect::StayWithEffect(
        State::Root,
        RenderPolicy::EditOrReply,
        vec![UiEffect::ClearPrompt],
    );
    apply_effect(&routes, &TestScene, &ctx, &vp, &d, &sctx, eff)
        .await
        .unwrap();

    let st = d.get_or_default().await.unwrap();

    assert!(st.ui_get_input_prompt_message_id().is_none());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn apply_effect_edit_only_does_not_create_new_message() {
    let (addr, shutdown) = start_test_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap());
    let vp = Viewport::new(NoopStore);
    let d = dialogue();
    let ctx = TestAppCtx {
        bot,
        chat: ChatId(1),
    };

    let routes = telegram_botkit::router::compose::Builder::<
        TestAppCtx,
        SimpleSession,
        InMemStorage<SimpleSession>,
        NoopStore,
    >::new()
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene>()
        .msg_entry(|b, d, m, s| Box::pin(dummy_msg_entry(b, d, m, s))),
    )
    .build()
    .unwrap();

    // First create a message
    let view = View {
        text: "menu".into(),
        markup: Some(teloxide::types::InlineKeyboardMarkup::new(vec![vec![]])),
        parse_mode: None,
        disable_web_page_preview: None,
    };

    vp.apply_view(
        &ctx.bot,
        ctx.chat,
        &d,
        &view,
        RenderPolicy::EditOrReply,
        None,
    )
    .await
    .unwrap();

    let before = d
        .get_or_default()
        .await
        .unwrap()
        .ui_get_last_action_message_id();

    let sctx = Ctx {
        user_id: ctx.user_id(),
    };
    let eff = Effect::Stay(State::Root, RenderPolicy::EditOnly);
    apply_effect(&routes, &TestScene, &ctx, &vp, &d, &sctx, eff)
        .await
        .unwrap();
    let after = d
        .get_or_default()
        .await
        .unwrap()
        .ui_get_last_action_message_id();
    assert_eq!(before, after);
    let _ = shutdown.send(());
}

#[tokio::test]
async fn apply_effect_noop_does_not_change_last_message() {
    let (addr, shutdown) = start_test_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap());
    let vp = Viewport::new(NoopStore);
    let d = dialogue();
    let ctx = TestAppCtx {
        bot,
        chat: ChatId(1),
    };

    let routes = telegram_botkit::router::compose::Builder::<
        TestAppCtx,
        SimpleSession,
        InMemStorage<SimpleSession>,
        NoopStore,
    >::new()
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene>()
        .msg_entry(|b, d, m, s| Box::pin(dummy_msg_entry(b, d, m, s))),
    )
    .build()
    .unwrap();

    let sctx = Ctx {
        user_id: ctx.user_id(),
    };
    let before = d
        .get_or_default()
        .await
        .unwrap()
        .ui_get_last_action_message_id();

    apply_effect(&routes, &TestScene, &ctx, &vp, &d, &sctx, Effect::Noop)
        .await
        .unwrap();

    let after = d
        .get_or_default()
        .await
        .unwrap()
        .ui_get_last_action_message_id();
    assert_eq!(before, after);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn apply_effect_switch_scene_sets_active_scene_id() {
    let (addr, shutdown) = start_test_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap());
    let vp = Viewport::new(NoopStore);
    let d = dialogue();
    let ctx = TestAppCtx {
        bot,
        chat: ChatId(1),
    };

    let routes = telegram_botkit::router::compose::Builder::<
        TestAppCtx,
        SimpleSession,
        InMemStorage<SimpleSession>,
        NoopStore,
    >::new()
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene>()
        .msg_entry(|b, d, m, s| Box::pin(dummy_msg_entry(b, d, m, s))),
    )
    .route(
        telegram_botkit::router::compose::Builder::<
            TestAppCtx,
            SimpleSession,
            InMemStorage<SimpleSession>,
            NoopStore,
        >::scene::<TestScene2>()
        .msg_entry(
            |b: &Bot,
             d: &Dialogue<SimpleSession, InMemStorage<SimpleSession>>,
             m: &Message,
             s: &State| Box::pin(dummy_msg_entry(b, d, m, s)),
        ),
    )
    .build()
    .unwrap();

    let sctx = Ctx {
        user_id: ctx.user_id(),
    };
    let eff = Effect::SwitchScene(SceneSwitch {
        to_scene_id: TestScene2::ID,
    });

    apply_effect(&routes, &TestScene, &ctx, &vp, &d, &sctx, eff)
        .await
        .unwrap();

    let st = d.get_or_default().await.unwrap();
    assert_eq!(st.ui_get_active_scene_id().as_deref(), Some(TestScene2::ID));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn viewport_apply_view_prompt_sets_prompt_id_and_clears_reply_flag() {
    let (addr, shutdown) = start_test_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&format!("http://{addr}")).unwrap());
    let vp = Viewport::new(NoopStore);
    let d = dialogue();

    // pre-set reply_to_last_once
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_last_action_message_id(Some(10));
        s.ui_set_reply_to_last_once(true);
        d.update(s).await.unwrap();
    }

    let view = View {
        text: "prompt".into(),
        markup: None,
        parse_mode: None,
        disable_web_page_preview: None,
    };
    vp.apply_view(
        &bot,
        ChatId(1),
        &d,
        &view,
        RenderPolicy::SendNew,
        Some(MetaSpec {
            scene_id: TestScene::ID,
            scene_version: TestScene::VERSION,
            state_json: Some("{}".into()),
            state_ref: None,
            ttl_secs: SNAP_TTL_SECS,
        }),
    )
    .await
    .unwrap();

    let st = d.get_or_default().await.unwrap();
    assert!(st.ui_get_input_prompt_message_id().is_some());
    assert!(!st.ui_get_reply_to_last_once());

    let _ = shutdown.send(());
}
