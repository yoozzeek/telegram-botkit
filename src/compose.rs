use crate::{session, viewport};
use teloxide::dispatching::dialogue;
use teloxide::requests::Requester;

pub trait SceneLookup: Send + Sync {
    fn find_scene_for_callback(&self, data: &str) -> Option<(&'static str, u16)>;
    fn id_exists(&self, id: &str) -> bool;
}

#[async_trait::async_trait]
pub trait RouteDispatch: SceneLookup {
    async fn handle_cb<C, D, S, M>(
        &self,
        ctx: &C,
        vp: &viewport::Viewport<M>,
        d: &dialogue::Dialogue<D, S>,
        q: &teloxide::types::CallbackQuery,
    ) -> anyhow::Result<bool>
    where
        C: crate::router::AppCtx + Sync,
        <C::Bot as Requester>::AnswerCallbackQuery: Send,
        <C::Bot as Requester>::SendMessage: Send,
        <C::Bot as Requester>::EditMessageText: Send,
        <C::Bot as Requester>::DeleteMessage: Send,
        D: session::UiStore + Send + Sync,
        S: dialogue::Storage<D> + Send + Sync,
        <S as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
        M: viewport::store::Store + Send + Sync;

    async fn handle_msg<C, D, S, M>(
        &self,
        active_id: Option<&str>,
        ctx: &C,
        vp: &viewport::Viewport<M>,
        d: &dialogue::Dialogue<D, S>,
        m: &teloxide::types::Message,
    ) -> anyhow::Result<bool>
    where
        C: crate::router::AppCtx + Sync,
        <C::Bot as Requester>::SendMessage: Send,
        <C::Bot as Requester>::EditMessageText: Send,
        <C::Bot as Requester>::DeleteMessage: Send,
        D: session::UiStore + Send + Sync,
        S: dialogue::Storage<D> + Send + Sync,
        <S as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
        M: viewport::store::Store + Send + Sync;

    async fn switch_to_scene_by_id<C, D, S, M>(
        &self,
        to_scene_id: &str,
        ctx: &C,
        vp: &viewport::Viewport<M>,
        d: &dialogue::Dialogue<D, S>,
    ) -> anyhow::Result<bool>
    where
        C: crate::router::AppCtx + Sync,
        <C::Bot as Requester>::SendMessage: Send,
        <C::Bot as Requester>::EditMessageText: Send,
        <C::Bot as Requester>::DeleteMessage: Send,
        D: session::UiStore + Send + Sync,
        S: dialogue::Storage<D> + Send + Sync,
        <S as dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
        M: viewport::store::Store + Send + Sync;
}

pub type Routes = std::sync::Arc<dyn RouteDispatch>;

#[macro_export]
macro_rules! routes {
    // Accept entries: Type or Type(entry = path)
    ( [ $($rest:tt)* ] ) => { $crate::routes!($($rest)*); };
    ( $( $name:ident $( (entry = $entry:path) )? ),* $(,)? ) => {{
        struct __GeneratedRoutes;

        impl $crate::compose::SceneLookup for __GeneratedRoutes {
            fn find_scene_for_callback(&self, data: &str) -> Option<(&'static str, u16)> {
                #[allow(unused_mut)]
                let mut result: Option<(&'static str, u16)> = None;

                $(
                    {
                        let p = < $name as $crate::scene::Scene >::PREFIX;
                        if let Some(rest) = data.strip_prefix(p) {
                            if rest.is_empty() || rest.starts_with(':') {
                                result = Some((
                                    < $name as $crate::scene::Scene >::ID,
                                    < $name as $crate::scene::Scene >::VERSION)
                                );
                            }
                        }
                    }
                )*

                result
            }

            fn id_exists(&self, id: &str) -> bool {
                false $(|| id == < $name as $crate::scene::Scene >::ID )*
            }
        }

        #[async_trait::async_trait]
        impl $crate::compose::RouteDispatch for __GeneratedRoutes {
            async fn handle_cb<C, D, S, M>(&self,
                ctx: &C,
                vp: &$crate::viewport::Viewport<M>,
                d: &teloxide::dispatching::dialogue::Dialogue<D, S>,
                q: &teloxide::types::CallbackQuery,
            ) -> anyhow::Result<bool>
            where
                C: $crate::router::AppCtx + Sync,
                <C::Bot as teloxide::requests::Requester>::AnswerCallbackQuery: Send,
                <C::Bot as teloxide::requests::Requester>::SendMessage: Send,
                <C::Bot as teloxide::requests::Requester>::EditMessageText: Send,
                <C::Bot as teloxide::requests::Requester>::DeleteMessage: Send,
                D: $crate::session::UiStore + Send + Sync,
                S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
                <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
                M: $crate::viewport::store::Store + Send + Sync,
            {
                let data = q.data.as_deref().unwrap_or("");

                $(
                    {
                        let p = < $name as $crate::scene::Scene >::PREFIX;
                        if let Some(rest) = data.strip_prefix(p) {
                            if rest.is_empty() || rest.starts_with(':') {
                                return $crate::router::common::run_cb(
                                    &$name,
                                    self,
                                    ctx,
                                    vp,
                                    d,
                                    q,
                                ).await;
                            }
                        }
                    }
                )*

                Ok(false)
            }

            async fn handle_msg<C, D, S, M>(&self,
                active_id: Option<&str>,
                ctx: &C,
                vp: &$crate::viewport::Viewport<M>,
                d: &teloxide::dispatching::dialogue::Dialogue<D, S>,
                m: &teloxide::types::Message,
            ) -> anyhow::Result<bool>
            where
                C: $crate::router::AppCtx + Sync,
                <C::Bot as teloxide::requests::Requester>::SendMessage: Send,
                <C::Bot as teloxide::requests::Requester>::EditMessageText: Send,
                <C::Bot as teloxide::requests::Requester>::DeleteMessage: Send,
                D: $crate::session::UiStore + Send + Sync,
                S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
                <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
                M: $crate::viewport::store::Store + Send + Sync,
            {
                let Some(id) = active_id else { return Ok(false) };

                match id {
                    $(
                        < $name as $crate::scene::Scene >::ID => {
                            $crate::router::common::run_msg(
                                &$name,
                                self,
                                $crate::routes!(@entry_opt2 $name $( $entry )? ),
                                ctx,
                                vp,
                                d,
                                m,
                            ).await
                        }
                    ),*
                    , _ => Ok(false)
                }
            }

            async fn switch_to_scene_by_id<C, D, S, M>(&self,
                to_scene_id: &str,
                ctx: &C,
                vp: &$crate::viewport::Viewport<M>,
                d: &teloxide::dispatching::dialogue::Dialogue<D, S>,
            ) -> anyhow::Result<bool>
            where
                C: $crate::router::AppCtx + Sync,
                <C::Bot as teloxide::requests::Requester>::SendMessage: Send,
                <C::Bot as teloxide::requests::Requester>::EditMessageText: Send,
                <C::Bot as teloxide::requests::Requester>::DeleteMessage: Send,
                D: $crate::session::UiStore + Send + Sync,
                S: teloxide::dispatching::dialogue::Storage<D> + Send + Sync,
                <S as teloxide::dispatching::dialogue::Storage<D>>::Error: std::fmt::Debug + Send,
                M: $crate::viewport::store::Store + Send + Sync,
            {
                match to_scene_id {
                    $(
                        < $name as $crate::scene::Scene >::ID => {
                            $crate::router::common::init_and_render(
                                &$name,
                                ctx,
                                vp,
                                d,
                            ).await?;
                            Ok(true)
                        }
                    ),*
                    , _ => Ok(false)
                }
            }
        }

        __GeneratedRoutes
    }};

    // Helpers: entry handler option
    (@entry_opt2 $name:ident) => {
        None::<fn(
            &C::Bot, &teloxide::dispatching::dialogue::Dialogue<D, S>,
            &teloxide::types::Message,
            &<$name as $crate::scene::Scene>::State) -> std::pin::Pin<
                Box<dyn std::future::Future<
                    Output = Option<<$name as $crate::scene::Scene>::State>> + Send>
                >
            >
    };
    (@entry_opt2 $name:ident $entry:path) => {
        Some(|bot, d, m, cur| Box::pin($entry(bot, d, m, cur)))
    };
}
