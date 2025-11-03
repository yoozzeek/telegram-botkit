use telegram_botkit::scene::*;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct DummyState {
    v: u32,
}

struct Dummy;

impl Scene for Dummy {
    const VERSION: u16 = 1;
    const ID: &'static str = "dummy";
    const PREFIX: &'static str = "d";

    type State = DummyState;
    type Event = ();

    fn init(&self, _ctx: &Ctx) -> Self::State {
        DummyState { v: 1 }
    }

    fn render(&self, _ctx: &Ctx, _s: &Self::State) -> View {
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

#[test]
fn snapshot_restore_ok() {
    let d = Dummy;
    let s = DummyState { v: 42 };
    let (json, _ref) = d.snapshot(&s);
    let snap = Snapshot {
        scene_version: Dummy::VERSION,
        scene_id: Dummy::ID,
        state_json: json.as_deref(),
        state_checksum: None,
    };
    let r = d.restore(snap).unwrap();

    assert_eq!(r.v, 42);
}

#[test]
fn snapshot_restore_bad_id() {
    let d = Dummy;
    let s = DummyState { v: 7 };
    let (json, _) = d.snapshot(&s);
    let snap = Snapshot {
        scene_version: Dummy::VERSION,
        scene_id: "wrong",
        state_json: json.as_deref(),
        state_checksum: None,
    };

    assert!(d.restore(snap).is_none());
}

#[test]
fn snapshot_restore_checksum_mismatch() {
    let d = Dummy;
    let s = DummyState { v: 7 };
    let (json, _) = d.snapshot(&s);
    let snap = Snapshot {
        scene_version: Dummy::VERSION,
        scene_id: Dummy::ID,
        state_json: json.as_deref(),
        state_checksum: Some("badhash"),
    };

    assert!(d.restore(snap).is_none());
}
