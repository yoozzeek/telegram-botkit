use crate::viewport::store::Store;
use redis::{AsyncCommands, aio::ConnectionManager};
use teloxide::types::ChatId;

#[cfg(feature = "encryption")]
use {
    base64::{Engine as _, engine::general_purpose::STANDARD as B64},
    chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::Aead, aead::KeyInit},
    rand::RngCore,
};

#[derive(Clone)]
pub struct RedisStore {
    redis: ConnectionManager,
    namespace: String,
    #[cfg(feature = "encryption")]
    enc_key: Option<[u8; 32]>,
}

impl RedisStore {
    /// Create store with default
    /// namespace, no encryption.
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        Self::new_with(redis_url, "tg:msgmeta", None).await
    }

    /// Create store with custom namespace
    /// and optional AEAD key (32 bytes).
    pub async fn new_with(
        redis_url: &str,
        namespace: impl Into<String>,
        enc_key: Option<&[u8]>,
    ) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let redis = client.get_connection_manager().await?;
        let namespace = namespace.into();

        #[cfg(feature = "encryption")]
        let enc_key = if let Some(k) = enc_key {
            if k.len() != 32 {
                anyhow::bail!("encryption key must be 32 bytes for ChaCha20-Poly1305");
            }

            let mut arr = [0u8; 32];
            arr.copy_from_slice(k);

            Some(arr)
        } else {
            None
        };

        Ok(Self {
            redis,
            namespace,
            #[cfg(feature = "encryption")]
            enc_key,
        })
    }

    fn key(&self, chat: ChatId, mid: i32) -> String {
        format!("{}:{}:{}", self.namespace, chat.0, mid)
    }

    #[cfg(feature = "encryption")]
    fn encrypt(&self, chat: ChatId, mid: i32, plaintext_json: &str) -> anyhow::Result<String> {
        let Some(key_bytes) = self.enc_key.as_ref() else {
            return Ok(plaintext_json.to_string());
        };
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));

        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);
        let aad = format!("ns={};chat={};mid={}", self.namespace, chat.0, mid);
        let ct = cipher
            .encrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: plaintext_json.as_bytes(),
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|e| anyhow::anyhow!("encrypt failed: {e}"))?;
        let wrapped = serde_json::json!({
            "enc": "cc20p1305",
            "nonce": B64.encode(nonce_bytes),
            "ct": B64.encode(ct),
        });

        Ok(wrapped.to_string())
    }

    #[cfg(feature = "encryption")]
    fn try_decrypt(&self, chat: ChatId, mid: i32, val: &str) -> anyhow::Result<String> {
        // Detect encrypted wrapper
        let Ok(v) = serde_json::from_str::<serde_json::Value>(val) else {
            return Ok(val.to_string());
        };
        let Some(enc) = v.get("enc").and_then(|s| s.as_str()) else {
            return Ok(val.to_string());
        };

        if enc != "cc20p1305" {
            return Ok(val.to_string());
        }

        let Some(key_bytes) = self.enc_key.as_ref() else {
            // Can't decrypt without key
            anyhow::bail!("encrypted value but no key configured");
        };
        let nonce_b64 = v.get("nonce").and_then(|s| s.as_str()).unwrap_or("");
        let ct_b64 = v.get("ct").and_then(|s| s.as_str()).unwrap_or("");
        let nonce_vec = B64.decode(nonce_b64)?;
        let ct_vec = B64.decode(ct_b64)?;

        if nonce_vec.len() != 12 {
            anyhow::bail!("bad nonce length");
        }

        let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
        let aad = format!("ns={};chat={};mid={}", self.namespace, chat.0, mid);
        let pt = cipher
            .decrypt(
                Nonce::from_slice(&nonce_vec),
                chacha20poly1305::aead::Payload {
                    msg: &ct_vec,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|e| anyhow::anyhow!("decrypt failed: {e}"))?;

        Ok(String::from_utf8(pt).unwrap_or_default())
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
        let json = serde_json::to_string(&meta)?;

        #[cfg(feature = "encryption")]
        let val = self.encrypt(chat, mid, &json)?;
        #[cfg(not(feature = "encryption"))]
        let val = json;

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
            #[cfg(feature = "encryption")]
            let raw = self.try_decrypt(chat, mid, &v)?;
            #[cfg(not(feature = "encryption"))]
            let raw = v;

            Ok(serde_json::from_str(&raw).ok())
        } else {
            Ok(None)
        }
    }
}
