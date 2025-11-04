use telegram_botkit::router::core::restore_state;
use telegram_botkit::scene::*;
use telegram_botkit::session::{SimpleSession, UiStore};
use telegram_botkit::viewport::{MessageMeta, MetaSpec, Viewport, store::Store};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};
use teloxide::types::{ChatId, MessageId};

#[derive(Clone, Default)]
struct MapStore(Arc<Mutex<HashMap<(i64, i32), MessageMeta>>>);

#[async_trait::async_trait]
impl Store for MapStore {
    async fn save(&self, chat: ChatId, mid: i32, meta: MessageMeta) -> anyhow::Result<()> {
        self.0.lock().unwrap().insert((chat.0, mid), meta);
        Ok(())
    }

    async fn load(&self, chat: ChatId, mid: i32) -> anyhow::Result<Option<MessageMeta>> {
        Ok(self.0.lock().unwrap().get(&(chat.0, mid)).cloned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum State {
    Root,
    FromDialogue(u32),
    FromMeta(u32),
}

#[derive(Clone, Debug)]
enum Event {}

struct TestScene;

impl Scene for TestScene {
    const VERSION: u16 = 1;
    const ID: &'static str = "test";
    const PREFIX: &'static str = "t";

    type State = State;
    type Event = Event;

    fn init(&self, _ctx: &Ctx) -> Self::State {
        State::Root
    }

    fn render(&self, _ctx: &Ctx, _state: &Self::State) -> View {
        View {
            text: String::new(),
            markup: None,
            parse_mode: None,
            disable_web_page_preview: None,
        }
    }

    fn update(&self, _ctx: &Ctx, s: &Self::State, _e: Self::Event) -> Effect<Self::State> {
        Effect::Stay(s.clone(), RenderPolicy::EditOnly)
    }

    fn bindings(&self) -> Bindings<Self::Event> {
        Bindings {
            msg: vec![],
            cb: vec![],
        }
    }
}

fn sctx() -> Ctx {
    Ctx { user_id: 1 }
}

fn dialogue() -> Dialogue<SimpleSession, InMemStorage<SimpleSession>> {
    let storage: Arc<InMemStorage<SimpleSession>> = InMemStorage::new();
    Dialogue::new(storage, ChatId(1))
}

#[tokio::test]
async fn restore_init_when_no_source() {
    let d = dialogue();
    let vp = Viewport::new(MapStore::default());
    let (st, label) = restore_state(&TestScene, &vp, &d, &sctx(), None).await;

    assert_eq!(st, State::Root);
    assert_eq!(label, "init");
}

#[tokio::test]
async fn restore_from_dialogue_snapshot() {
    let d = dialogue();
    // prepare dialogue with last action message and state json
    let mid = 42;
    let mut s = d.get_or_default().await.unwrap();
    s.ui_set_last_action_message_id(Some(mid));

    let from = State::FromDialogue(7);
    let json = serde_json::to_string(&from).unwrap();
    s.ui_set_scene_for_message(mid, json);
    d.update(s).await.unwrap();

    let vp = Viewport::new(MapStore::default());
    let (st, label) = restore_state(
        &TestScene,
        &vp,
        &d,
        &sctx(),
        Some((ChatId(1), MessageId(mid))),
    )
    .await;

    assert_eq!(st, State::FromDialogue(7));
    assert_eq!(label, "dialogue");
}

#[tokio::test]
async fn restore_from_meta_snapshot() {
    let store = MapStore::default();
    let vp = Viewport::new(store.clone());
    let d = dialogue();

    let mid = 100;
    let from = State::FromMeta(9);
    let json = serde_json::to_string(&from).unwrap();

    // Save via public helper to compute checksum
    vp.save_meta_public(
        ChatId(1),
        mid,
        MetaSpec {
            scene_id: TestScene::ID,
            scene_version: TestScene::VERSION,
            state_json: Some(json),
            state_ref: None,
            ttl_secs: 60,
        },
    )
    .await
    .unwrap();

    let (st, label) = restore_state(
        &TestScene,
        &vp,
        &d,
        &sctx(),
        Some((ChatId(1), MessageId(mid))),
    )
    .await;
    assert_eq!(st, State::FromMeta(9));
    assert_eq!(label, "meta");
}

#[tokio::test]
async fn restore_meta_mismatch_falls_back_to_init() {
    let store = MapStore::default();
    let vp = Viewport::new(store.clone());
    let d = dialogue();

    let mid = 77;
    let from = State::FromMeta(11);
    let json = serde_json::to_string(&from).unwrap();

    // Save bad checksum directly through store
    let meta = MessageMeta {
        scene_id: TestScene::ID.into(),
        scene_version: TestScene::VERSION,
        state_json: Some(json),
        state_ref: None,
        state_checksum: Some("badhash".into()),
        created_at: 0,
        ttl_secs: 60,
    };
    store.save(ChatId(1), mid, meta).await.unwrap();

    let (st, label) = restore_state(
        &TestScene,
        &vp,
        &d,
        &sctx(),
        Some((ChatId(1), MessageId(mid))),
    )
    .await;
    assert_eq!(st, State::Root);
    assert_eq!(label, "mismatch");
}
