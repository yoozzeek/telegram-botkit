use crate::viewport::MessageMeta;
use teloxide::types::ChatId;

#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
    async fn save(&self, chat: ChatId, mid: i32, meta: MessageMeta) -> anyhow::Result<()>;
    async fn load(&self, chat: ChatId, mid: i32) -> anyhow::Result<Option<MessageMeta>>;
}

#[derive(Clone, Copy, Default)]
pub struct NoopStore;

#[async_trait::async_trait]
impl Store for NoopStore {
    async fn save(&self, _chat: ChatId, _mid: i32, _meta: MessageMeta) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load(&self, _chat: ChatId, _mid: i32) -> anyhow::Result<Option<MessageMeta>> {
        Ok(None)
    }
}
