use crate::viewport::store::Store;
use redis::{AsyncCommands, aio::ConnectionManager};
use teloxide::types::ChatId;

#[derive(Clone)]
pub struct RedisStore {
    redis: ConnectionManager,
    key_prefix: String,
}

impl RedisStore {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis = client.get_connection_manager().await?;

        Ok(Self {
            redis,
            key_prefix: "tg:msgmeta".to_string(),
        })
    }

    fn key(&self, chat: ChatId, mid: i32) -> String {
        format!("{}:{}:{}", self.key_prefix, chat.0, mid)
    }
}

#[async_trait::async_trait]
impl Store for RedisStore {
    async fn save(
        &self,
        chat: ChatId,
        mid: i32,
        meta: crate::viewport::MessageMeta,
    ) -> anyhow::Result<()> {
        let key = self.key(chat, mid);
        let val = serde_json::to_string(&meta)?;
        let mut conn = self.redis.clone();

        let _: () = conn.set_ex(key, val, meta.ttl_secs as u64).await?;

        Ok(())
    }

    async fn load(
        &self,
        chat: ChatId,
        mid: i32,
    ) -> anyhow::Result<Option<crate::viewport::MessageMeta>> {
        let key = self.key(chat, mid);
        let mut conn = self.redis.clone();

        let val: Option<String> = conn.get(key).await?;

        if let Some(v) = val {
            Ok(serde_json::from_str(&v).ok())
        } else {
            Ok(None)
        }
    }
}
