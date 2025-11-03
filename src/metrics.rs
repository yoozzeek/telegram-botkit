use std::sync::Arc;
use std::sync::OnceLock;

pub trait MetricsHook: Send + Sync + 'static {
    fn router_handle(&self, _event: &'static str, _chat: i64, _user: i64) {}
    fn restore_state(&self, _scene_id: &'static str, _path: &'static str) {}
    fn apply_effect(&self, _scene_id: &'static str, _effect: &'static str) {}
    fn apply_view(&self, _policy: &'static str) {}
}

static METRICS_HOOK: OnceLock<Arc<dyn MetricsHook>> = OnceLock::new();

pub fn set_hook(h: Arc<dyn MetricsHook>) -> Result<(), Arc<dyn MetricsHook>> {
    METRICS_HOOK.set(h)
}

#[inline]
pub fn router_handle(event: &'static str, chat: i64, user: i64) {
    if let Some(h) = METRICS_HOOK.get() {
        h.router_handle(event, chat, user);
    }
}

#[inline]
pub fn restore_state(scene_id: &'static str, path: &'static str) {
    if let Some(h) = METRICS_HOOK.get() {
        h.restore_state(scene_id, path);
    }
}

#[inline]
pub fn apply_effect(scene_id: &'static str, effect: &'static str) {
    if let Some(h) = METRICS_HOOK.get() {
        h.apply_effect(scene_id, effect);
    }
}

#[inline]
pub fn apply_view(policy: &'static str) {
    if let Some(h) = METRICS_HOOK.get() {
        h.apply_view(policy);
    }
}
