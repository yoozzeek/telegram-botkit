use opentelemetry::metrics::{Counter, Meter};
use opentelemetry::{InstrumentationScope, KeyValue, global};
use std::sync::OnceLock;

static ROUTER_COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
static RESTORE_COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
static EFFECT_COUNTER: OnceLock<Counter<u64>> = OnceLock::new();
static VIEW_COUNTER: OnceLock<Counter<u64>> = OnceLock::new();

fn meter() -> Meter {
    let scope = InstrumentationScope::builder(env!("CARGO_PKG_NAME"))
        .with_version(env!("CARGO_PKG_VERSION"))
        .build();
    global::meter_with_scope(scope)
}

#[inline]
pub fn router_handle(event: &'static str, chat: i64, user: i64) {
    let c = ROUTER_COUNTER.get_or_init(|| {
        meter()
            .u64_counter("router_handle")
            .with_description("router events")
            .build()
    });
    c.add(
        1,
        &[
            KeyValue::new("event", event),
            KeyValue::new("chat", chat),
            KeyValue::new("user", user),
        ],
    );
}

#[inline]
pub fn restore_state(scene_id: &'static str, path: &'static str) {
    let c = RESTORE_COUNTER.get_or_init(|| {
        meter()
            .u64_counter("restore_state")
            .with_description("restore path")
            .build()
    });
    c.add(
        1,
        &[
            KeyValue::new("scene", scene_id),
            KeyValue::new("path", path),
        ],
    );
}

#[inline]
pub fn apply_effect(scene_id: &'static str, effect: &'static str) {
    let c = EFFECT_COUNTER.get_or_init(|| {
        meter()
            .u64_counter("apply_effect")
            .with_description("effect kind")
            .build()
    });
    c.add(
        1,
        &[
            KeyValue::new("scene", scene_id),
            KeyValue::new("effect", effect),
        ],
    );
}

#[inline]
pub fn apply_view(policy: &'static str) {
    let c = VIEW_COUNTER.get_or_init(|| {
        meter()
            .u64_counter("apply_view")
            .with_description("render policy")
            .build()
    });
    c.add(1, &[KeyValue::new("policy", policy)]);
}
