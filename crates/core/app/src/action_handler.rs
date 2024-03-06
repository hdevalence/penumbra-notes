use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use cnidarium::{StateRead, StateWrite};

mod actions;
mod transaction;

/// Stub: to be replaced with impls of cnidarium_component::ActionHandler
///
/// This trait should move to that crate, but the orphan rules make it tricky to
/// move it before we finish splitting all the crates: if we move the trait now,
/// existing impls in this crate on foreign types will all break. But without
/// moving it, we can't start splitting up the crate at all.  Solution:
/// duplicate the trait here and there, and provide a generic impl of this trait
/// for anything implementing the copy of the trait in the other crate.  Later,
/// we can delete this trait entirely.
#[async_trait]
pub trait ActionHandler {
    type CheckStatelessContext: Clone + Send + Sync + 'static;
    async fn check_stateless(&self, context: Self::CheckStatelessContext) -> Result<()>;
    async fn check_stateful<S: StateRead + 'static>(&self, state: Arc<S>) -> Result<()>;
    async fn execute<S: StateWrite>(&self, state: S) -> Result<()>;
}

use cnidarium_component::ActionHandler as ComponentActionHandler;

#[async_trait]
impl<'a, T: ComponentActionHandler + Sync> ActionHandler for crate::Compat<'a, T> {
    type CheckStatelessContext = T::CheckStatelessContext;
    async fn check_stateless(&self, context: Self::CheckStatelessContext) -> Result<()> {
        ComponentActionHandler::check_stateless(self.0, context).await
    }
    async fn check_stateful<S: StateRead + 'static>(&self, state: Arc<S>) -> Result<()> {
        ComponentActionHandler::check_historical(self.0, state).await
    }
    async fn execute<S: StateWrite>(&self, state: S) -> Result<()> {
        ComponentActionHandler::check_and_execute(self.0, state).await
    }
}
