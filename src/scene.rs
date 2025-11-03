use teloxide::types::{InlineKeyboardMarkup, ParseMode};

pub trait ActionCodec: Sized {
    fn encode(&self, prefix: &str) -> String;
    fn decode(prefix: &str, s: &str) -> Option<Self>;
}

#[derive(Clone, Debug)]
pub struct Ctx {
    pub user_id: i64,
}

#[derive(Clone, Debug)]
pub struct View {
    pub text: String,
    pub markup: Option<InlineKeyboardMarkup>,
    pub parse_mode: Option<ParseMode>,
    pub disable_web_page_preview: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub enum RenderPolicy {
    EditOrReply,
    SendNew,
    EditOnly,
}

#[derive(Clone, Debug)]
pub enum Effect<S> {
    Stay(S, RenderPolicy),
    StayWithEffect(S, RenderPolicy, Vec<UiEffect>),
    SwitchScene(SceneSwitch),
    Noop,
    NoopWithEffect(Vec<UiEffect>),
}

#[derive(Clone, Debug)]
pub enum UiEffect {
    Notification {
        text_md: String,
        ttl_secs: Option<u64>,
    },
    ClearPrompt,
}

#[derive(Clone, Debug)]
pub struct SceneSwitch {
    pub to_scene_id: &'static str,
}

pub enum MsgPattern {
    AnyText,
    Command(&'static str),
    Regex(&'static str),
}

pub enum CbKey {
    Exact(&'static str),
    Prefix(&'static str),
}

pub struct MsgBinding<E> {
    pub pattern: MsgPattern,
    pub to_event: fn(&teloxide::types::Message) -> Option<E>,
}

pub struct CbBinding<E> {
    pub key: CbKey,
    pub to_event: fn(&teloxide::types::CallbackQuery) -> Option<E>,
}

pub struct Bindings<E> {
    pub msg: Vec<MsgBinding<E>>,
    pub cb: Vec<CbBinding<E>>,
}

#[derive(Clone, Debug)]
pub struct Snapshot<'a> {
    pub scene_version: u16,
    pub scene_id: &'a str,
    pub state_json: Option<&'a str>,
    pub state_checksum: Option<&'a str>,
}

pub trait Scene: Send + Sync + 'static {
    const VERSION: u16;
    const ID: &'static str;
    const PREFIX: &'static str;

    type State: serde::de::DeserializeOwned + serde::Serialize + Send + Clone + Eq + std::fmt::Debug;
    type Event: Clone + std::fmt::Debug;

    fn init(&self, _ctx: &Ctx) -> Self::State;

    fn render(&self, _ctx: &Ctx, _state: &Self::State) -> View;

    fn update(&self, _ctx: &Ctx, state: &Self::State, _event: Self::Event) -> Effect<Self::State>;

    fn bindings(&self) -> Bindings<Self::Event>;

    fn snapshot(&self, state: &Self::State) -> (Option<String>, Option<String>) {
        match serde_json::to_string(state) {
            Ok(s) => (Some(s), None),
            Err(_) => (None, None),
        }
    }

    fn restore(&self, snap: Snapshot<'_>) -> Option<Self::State> {
        if snap.scene_id != Self::ID {
            return None;
        }

        let json = snap.state_json?;
        if let Some(cs) = snap.state_checksum {
            let ours = {
                let h = blake3::hash(json.as_bytes());
                hex::encode(h.as_bytes())
            };

            if cs != ours {
                return None;
            }
        }

        serde_json::from_str::<Self::State>(json).ok()
    }
}
