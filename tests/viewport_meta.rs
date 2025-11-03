use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use telegram_botkit::viewport::{MessageMeta, MetaSpec, Viewport, store::Store};
use teloxide::types::ChatId;

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

#[tokio::test]
async fn mapstore_roundtrip() {
    let store = MapStore::default();
    let chat = ChatId(1);
    let meta = MessageMeta {
        scene_id: "home".to_string(),
        scene_version: 1,
        state_json: Some("{\"state\":\"Root\"}".to_string()),
        state_ref: None,
        state_checksum: Some("deadbeef".into()),
        created_at: 0,
        ttl_secs: 60,
    };

    // Save & Load through store directly
    store.save(chat, 42, meta.clone()).await.unwrap();

    let vp = Viewport::new(store);
    vp.save_meta_public(
        chat,
        42,
        MetaSpec {
            scene_id: "home",
            scene_version: 1,
            state_json: Some("{\"state\":\"Root\"}".to_string()),
            state_ref: None,
            ttl_secs: 60,
        },
    )
    .await
    .unwrap();

    let got = vp.load_meta(chat, 42).await.unwrap();
    let meta = got.expect("meta");
    assert_eq!(meta.scene_id, "home");
    assert!(meta.state_checksum.is_some());
}
