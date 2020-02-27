use crate::WrapError;
use diesel::r2d2::{ConnectionManager, PoolError};
use diesel::Connection;
use r2d2::{Builder, PooledConnection};
use roa_core::{async_trait, Context, State};
use std::time::Duration;

type Pool<Conn> = r2d2::Pool<ConnectionManager<Conn>>;

pub type WrapConnection<Conn> = PooledConnection<ConnectionManager<Conn>>;

pub trait MakePool<Conn>
where
    Conn: Connection + 'static,
{
    fn make(url: impl Into<String>) -> Result<Pool<Conn>, PoolError> {
        r2d2::Pool::new(ConnectionManager::<Conn>::new(url))
    }

    fn builder() -> Builder<ConnectionManager<Conn>> {
        r2d2::Pool::builder()
    }

    fn pool(&self) -> &Pool<Conn>;
}

#[async_trait(?Send)]
pub trait AsyncPool<Conn>
where
    Conn: Connection + 'static,
{
    async fn get_conn(&self) -> Result<WrapConnection<Conn>, WrapError>;

    async fn get_timeout(
        &self,
        timeout: Duration,
    ) -> Result<WrapConnection<Conn>, WrapError>;

    async fn pool_state(&self) -> r2d2::State;
}

#[async_trait(?Send)]
impl<S, Conn> AsyncPool<Conn> for Context<S>
where
    S: State + MakePool<Conn>,
    Conn: Connection + 'static,
{
    async fn get_conn(&self) -> Result<WrapConnection<Conn>, WrapError> {
        let pool = self.pool().clone();
        Ok(self.exec().spawn_blocking(move || pool.get()).await?)
    }

    async fn get_timeout(
        &self,
        timeout: Duration,
    ) -> Result<WrapConnection<Conn>, WrapError> {
        let pool = self.pool().clone();
        Ok(self
            .exec()
            .spawn_blocking(move || pool.get_timeout(timeout))
            .await?)
    }

    async fn pool_state(&self) -> r2d2::State {
        let pool = self.pool().clone();
        self.exec().spawn_blocking(move || pool.state()).await
    }
}
