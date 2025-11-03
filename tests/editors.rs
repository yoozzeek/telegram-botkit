use telegram_botkit::session::{SimpleSession, UiStore};
use telegram_botkit::ui::{editors, strings};
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
use std::sync::Arc;
use tokio::net::TcpListener;

async fn handle(_req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let body = serde_json::json!({"ok": true, "result": true});
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
        .unwrap())
}

async fn start_server() -> (String, tokio::sync::oneshot::Sender<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, mut rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => break,
                res = listener.accept() => {
                    let (stream, _) = res.unwrap();
                    let io = TokioIo::new(stream);
                    let _ = http1::Builder::new().serve_connection(io, service_fn(handle)).await;
                }
            }
        }
    });

    (format!("http://{addr}"), tx)
}

fn dialogue() -> Dialogue<SimpleSession, InMemStorage<SimpleSession>> {
    let storage: Arc<InMemStorage<SimpleSession>> = InMemStorage::new();
    Dialogue::new(storage, ChatId(1))
}

#[tokio::test]
async fn u64_valid_no_text_false() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();

    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let m_no_text: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 4, "date": 0, "chat": {"id": 1, "type": "private"}
    }))
    .unwrap();

    let ok = editors::edit_u64_valid(
        &bot,
        &d,
        &m_no_text,
        |v| v >= 10,
        |_v| {},
        |v| format!("ok {v}"),
        strings::ERR_INVALID_NUMBER,
    )
    .await;
    assert!(!ok);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn u64_valid_ok() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();

    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let m_ok: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 5, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "123"
    }))
    .unwrap();

    let ok = editors::edit_u64_valid(
        &bot,
        &d,
        &m_ok,
        |v| v >= 10,
        |_v| {},
        |v| format!("ok {v}"),
        strings::ERR_INVALID_NUMBER,
    )
    .await;
    assert!(ok);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn time_min_false() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let m_bad: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 6, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "0s"
    }))
    .unwrap();

    let ok = editors::edit_time_secs(
        &bot,
        &d,
        &m_bad,
        10,
        |_v| {},
        |v| format!("ok {v}"),
        strings::ERR_INVALID_TIME_FORMAT,
    )
    .await;
    assert!(!ok);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn base58_ok() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let m58_ok: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 7, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "11111111111111111111111111111111"
    })).unwrap();
    let mut captured: Option<String> = None;

    let ok = editors::edit_base58_address(
        &bot,
        &d,
        &m58_ok,
        |addr| captured = Some(addr),
        |a| format!("ok {a}"),
        "err",
    )
    .await;
    assert!(ok);
    assert!(captured.is_some());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn base58_bad() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let m58_bad: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 8, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "illeg@l"
    }))
    .unwrap();

    let ok =
        editors::edit_base58_address(&bot, &d, &m58_bad, |_a| {}, |_a| "ok".into(), "err").await;
    assert!(!ok);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn pct_pos_ok() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let mpos: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 9, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "15"
    }))
    .unwrap();

    let mut got_bp: Option<u64> = None;
    let ok = editors::edit_percent_positive(
        &bot,
        &d,
        &mpos,
        300..=10_000,
        |bp| got_bp = Some(bp),
        |bp| format!("ok {bp}"),
        strings::ERR_INVALID_PERCENT_RANGE,
    )
    .await;

    assert!(ok);
    assert_eq!(got_bp, Some(1500));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn pct_pos_neg_sign_false() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let mpos_bad: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 10, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "-5"
    }))
    .unwrap();

    let ok = editors::edit_percent_positive(
        &bot,
        &d,
        &mpos_bad,
        300..=10_000,
        |_bp| {},
        |bp| format!("ok {bp}"),
        strings::ERR_INVALID_PERCENT_RANGE,
    )
    .await;
    assert!(!ok);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn pct_neg_ok() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let mneg: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 11, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "-3"
    }))
    .unwrap();
    let mut got_bp2: Option<u64> = None;

    let ok = editors::edit_percent_negative(
        &bot,
        &d,
        &mneg,
        1..=10_000,
        |bp| got_bp2 = Some(bp),
        |bp| format!("ok {bp}"),
        strings::ERR_INVALID_PERCENT_RANGE,
    )
    .await;
    assert!(ok);
    assert_eq!(got_bp2, Some(300));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn pct_neg_plus_sign_false() {
    let (url, shutdown) = start_server().await;
    let bot = Bot::with_client("TEST", reqwest::Client::new())
        .set_api_url(reqwest::Url::parse(&url).unwrap());
    let d: Dialogue<SimpleSession, InMemStorage<SimpleSession>> = dialogue();
    {
        let mut s = d.get_or_default().await.unwrap();
        s.ui_set_input_prompt_message_id(Some(1));
        d.update(s).await.unwrap();
    }

    let mneg_bad: teloxide::types::Message = serde_json::from_value(serde_json::json!({
        "message_id": 12, "date": 0, "chat": {"id": 1, "type": "private"}, "text": "+5"
    }))
    .unwrap();

    let ok = editors::edit_percent_negative(
        &bot,
        &d,
        &mneg_bad,
        1..=10_000,
        |_bp| {},
        |bp| format!("ok {bp}"),
        strings::ERR_INVALID_PERCENT_RANGE,
    )
    .await;
    assert!(!ok);

    let _ = shutdown.send(());
}
