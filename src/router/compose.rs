use crate::prelude::UiStore;
use crate::router;
use crate::router::AppCtx;
use crate::router::core::{CbEntryDyn, MsgEntryDyn, init_and_render, run_cb, run_msg};
use crate::scene::{CbKey, Scene};
use crate::viewport::{Viewport, store};

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use teloxide::dispatching::dialogue::{self, Dialogue};
use teloxide::prelude::Requester;
use teloxide::types::{CallbackQuery, Message};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComposeError {
    #[error("duplicate scene id: {0}")]
    DuplicateId(&'static str),
    #[error("duplicate scene prefix: {0}")]
    DuplicatePrefix(&'static str),
}

#[async_trait::async_trait]
pub trait RouterDispatch<C, D, St, M>: Send + Sync
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    async fn switch_to_scene_by_id(
        &self,
        id: &str,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
    ) -> anyhow::Result<bool>;

    async fn handle_msg(
        &self,
        active: Option<&str>,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        m: &Message,
    ) -> anyhow::Result<bool>;

    async fn handle_cb(
        &self,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        q: &CallbackQuery,
    ) -> anyhow::Result<bool>;
}

pub trait SceneLookup: Send + Sync {
    fn find_scene_for_callback(&self, data: &str) -> Option<(&'static str, u16)>;
}

#[async_trait::async_trait]
trait RouteFns<C, D, St, M>: Send + Sync
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    fn id(&self) -> &'static str;

    fn prefix(&self) -> &'static str;

    fn version(&self) -> u16;

    fn matches_cb(&self, data: &str) -> bool;

    async fn handle_msg(
        &self,
        router: &Routes<C, D, St, M>,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        m: &Message,
    ) -> anyhow::Result<bool>;

    async fn handle_cb(
        &self,
        router: &Routes<C, D, St, M>,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        q: &CallbackQuery,
    ) -> anyhow::Result<bool>;

    async fn init_and_render(
        &self,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
    ) -> anyhow::Result<()>;
}

struct SceneRoute<S, C, D, St, M>
where
    S: Scene,
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    scene: S,
    msg_entry: Option<Box<MsgEntryDyn<S, C, D, St>>>,
    cb_entry: Option<Box<CbEntryDyn<S, C, D, St>>>,
    _pd: PhantomData<(C, D, St, M)>,
}

#[async_trait::async_trait]
impl<S, C, D, St, M> RouteFns<C, D, St, M> for SceneRoute<S, C, D, St, M>
where
    S: Scene + Send + Sync + 'static,
    C: AppCtx + Send + Sync + 'static,
    D: UiStore + Send + Sync + 'static,
    St: dialogue::Storage<D> + Send + Sync + 'static,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync + 'static,
    <C::Bot as Requester>::SendMessage: Send,
    <C::Bot as Requester>::EditMessageText: Send,
    <C::Bot as Requester>::DeleteMessage: Send,
    <C::Bot as Requester>::AnswerCallbackQuery: Send,
{
    fn id(&self) -> &'static str {
        S::ID
    }

    fn prefix(&self) -> &'static str {
        S::PREFIX
    }

    fn version(&self) -> u16 {
        S::VERSION
    }

    fn matches_cb(&self, data: &str) -> bool {
        if data.starts_with(S::PREFIX) {
            return true;
        }

        let bindings = self.scene.bindings();
        for b in bindings.cb.iter() {
            match b.key {
                CbKey::Exact(k) => {
                    if data == k {
                        return true;
                    }
                }
                CbKey::Prefix(p) => {
                    if data.starts_with(p) {
                        return true;
                    }
                }
            }
        }

        false
    }

    async fn handle_msg(
        &self,
        router: &Routes<C, D, St, M>,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        m: &Message,
    ) -> anyhow::Result<bool> {
        run_msg(
            &self.scene,
            router,
            self.msg_entry.as_deref(),
            ctx,
            vp,
            d,
            m,
        )
        .await
    }

    async fn handle_cb(
        &self,
        router: &Routes<C, D, St, M>,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        q: &CallbackQuery,
    ) -> anyhow::Result<bool> {
        run_cb(&self.scene, router, self.cb_entry.as_deref(), ctx, vp, d, q).await
    }

    async fn init_and_render(
        &self,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
    ) -> anyhow::Result<()> {
        init_and_render(&self.scene, ctx, vp, d).await
    }
}

pub struct Routes<C, D, St, M>
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
{
    items: Vec<Box<dyn RouteFns<C, D, St, M>>>,
    idx_by_id: HashMap<&'static str, usize>,
    prefixes: Vec<(&'static str, usize)>,
}

impl<C, D, St, M> Routes<C, D, St, M>
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    fn new(items: Vec<Box<dyn RouteFns<C, D, St, M>>>) -> Result<Self, ComposeError> {
        let mut idx_by_id = HashMap::new();
        let mut seen_ids = HashSet::new();
        let mut seen_prefix = HashSet::new();
        let mut prefixes = Vec::with_capacity(items.len());

        for (i, it) in items.iter().enumerate() {
            let id = it.id();
            if !seen_ids.insert(id) {
                return Err(ComposeError::DuplicateId(id));
            }

            idx_by_id.insert(id, i);

            let pf = it.prefix();
            if !seen_prefix.insert(pf) {
                return Err(ComposeError::DuplicatePrefix(pf));
            }

            prefixes.push((pf, i));
        }

        Ok(Self {
            items,
            idx_by_id,
            prefixes,
        })
    }
}

#[async_trait::async_trait]
impl<C, D, St, M> RouterDispatch<C, D, St, M> for Routes<C, D, St, M>
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    async fn switch_to_scene_by_id(
        &self,
        id: &str,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
    ) -> anyhow::Result<bool> {
        match self.idx_by_id.get(id).copied() {
            Some(i) => {
                self.items[i].init_and_render(ctx, vp, d).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn handle_msg(
        &self,
        active: Option<&str>,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        m: &Message,
    ) -> anyhow::Result<bool> {
        if let Some(id) = active {
            if let Some(i) = self.idx_by_id.get(id).copied() {
                return self.items[i].handle_msg(self, ctx, vp, d, m).await;
            }
        }

        for it in &self.items {
            if it.handle_msg(self, ctx, vp, d, m).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn handle_cb(
        &self,
        ctx: &C,
        vp: &Viewport<M>,
        d: &Dialogue<D, St>,
        q: &CallbackQuery,
    ) -> anyhow::Result<bool> {
        let data = q.data.as_deref().unwrap_or("");

        if let Some((id, _ver)) = self.find_scene_for_callback(data) {
            if let Some(i) = self.idx_by_id.get(id).copied() {
                return self.items[i].handle_cb(self, ctx, vp, d, q).await;
            }
        }

        for it in &self.items {
            if it.handle_cb(self, ctx, vp, d, q).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl<C, D, St, M> SceneLookup for Routes<C, D, St, M>
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    fn find_scene_for_callback(&self, data: &str) -> Option<(&'static str, u16)> {
        if data.is_empty() {
            return None;
        }

        // Fast path by prefix
        for (pf, i) in &self.prefixes {
            if data.starts_with(pf) {
                return Some((self.items[*i].id(), self.items[*i].version()));
            }
        }

        // Check bindings
        for it in &self.items {
            if it.matches_cb(data) {
                return Some((it.id(), it.version()));
            }
        }

        None
    }
}

pub struct Builder<C, D, St, M>
where
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
{
    routes: Vec<Box<dyn RouteFns<C, D, St, M>>>,
}

impl<C, D, St, M> Builder<C, D, St, M>
where
    C: AppCtx + Send + Sync + 'static,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync + 'static,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn route<S>(mut self, sc: SceneBuilder<S, C, D, St, M>) -> Self
    where
        S: Scene + Send + Sync + 'static,
        <C::Bot as Requester>::SendMessage: Send,
        <C::Bot as Requester>::EditMessageText: Send,
        <C::Bot as Requester>::DeleteMessage: Send,
        <C::Bot as Requester>::AnswerCallbackQuery: Send,
    {
        let route = Box::new(SceneRoute::<S, C, D, St, M> {
            scene: sc.scene,
            msg_entry: sc.msg_entry,
            cb_entry: sc.cb_entry,
            _pd: PhantomData,
        });

        self.routes.push(route);

        self
    }

    pub fn build(self) -> Result<Routes<C, D, St, M>, ComposeError> {
        Routes::new(self.routes)
    }
}

impl<C, D, St, M> Default for Builder<C, D, St, M>
where
    C: AppCtx + Send + Sync + 'static,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync + 'static,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct SceneBuilder<S, C, D, St, M>
where
    S: Scene,
    C: AppCtx,
{
    scene: S,
    msg_entry: Option<Box<MsgEntryDyn<S, C, D, St>>>,
    cb_entry: Option<Box<CbEntryDyn<S, C, D, St>>>,
    _pd: PhantomData<(C, D, St, M)>,
}

impl<S, C, D, St, M> SceneBuilder<S, C, D, St, M>
where
    S: Scene + Send + Sync + 'static,
    C: AppCtx + Send + Sync + 'static,
    D: UiStore + Send + Sync + 'static,
    St: dialogue::Storage<D> + Send + Sync + 'static,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    pub fn msg_entry<F>(mut self, f: F) -> Self
    where
        for<'a> F: Fn(
                &'a <C as AppCtx>::Bot,
                &'a Dialogue<D, St>,
                &'a Message,
                &'a <S as Scene>::State,
            ) -> router::core::EntryFuture<'a, S>
            + Send
            + Sync
            + 'static,
    {
        self.msg_entry = Some(Box::new(f));
        self
    }

    pub fn cb_entry<F>(mut self, f: F) -> Self
    where
        for<'a> F: Fn(
                &'a <C as AppCtx>::Bot,
                &'a Dialogue<D, St>,
                &'a CallbackQuery,
                &'a <S as Scene>::State,
            ) -> router::core::EntryFuture<'a, S>
            + Send
            + Sync
            + 'static,
    {
        self.cb_entry = Some(Box::new(f));
        self
    }
}

pub fn scene<S, C, D, St, M>() -> SceneBuilder<S, C, D, St, M>
where
    S: Scene + Default,
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    SceneBuilder {
        scene: S::default(),
        msg_entry: None,
        cb_entry: None,
        _pd: PhantomData,
    }
}

pub fn scene_with<S, C, D, St, M>(scene: S) -> SceneBuilder<S, C, D, St, M>
where
    S: Scene,
    C: AppCtx + Send + Sync,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
{
    SceneBuilder {
        scene,
        msg_entry: None,
        cb_entry: None,
        _pd: PhantomData,
    }
}

impl<C, D, St, M> Builder<C, D, St, M>
where
    C: AppCtx + Send + Sync + 'static,
    D: UiStore + Send + Sync,
    St: dialogue::Storage<D> + Send + Sync + 'static,
    <St as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
    M: store::Store + Send + Sync,
{
    pub fn scene<S>() -> SceneBuilder<S, C, D, St, M>
    where
        S: Scene + Default,
    {
        scene::<S, C, D, St, M>()
    }

    pub fn scene_with<S>(scene: S) -> SceneBuilder<S, C, D, St, M>
    where
        S: Scene,
    {
        scene_with::<S, C, D, St, M>(scene)
    }
}
