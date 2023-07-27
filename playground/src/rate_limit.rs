use crate::{Error, Result};
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::Request;
use futures::future::{BoxFuture, Fuse, FusedFuture};
use futures::{Future, TryFuture};
use pin_project::pin_project;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tower::Service;

pub const MAX_NUM: u64 = u64::MAX >> 1;

#[derive(Debug, Clone)]
pub struct LimitInfo {
    rate: Rate,
    handler: LimitHandler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    num: u64,
    interval: Duration,
}

#[derive(Debug, Clone)]
pub enum LimitHandler {
    Wait,
    Fail,
}

#[derive(Debug, Clone)]
pub struct RateLimitService<S> {
    state: S,
    // TODO global
    specific: Arc<HashMap<SocketAddr, Limiter>>,
}

impl<S, B: 'static> Service<Request<B>> for RateLimitService<S>
where
    S: 'static + Service<Request<B>> + Clone + Send + Sync,
    Error: Into<S::Error>,
    ConnectInfo<SocketAddr>: FromRequestParts<S>,
    <ConnectInfo<SocketAddr> as FromRequestParts<S>>::Rejection: Into<S::Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = impl 'static + Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.state.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let state = self.state.clone();
        let specific = self.specific.clone();

        return async move {
            let (mut parts, body) = req.into_parts();
            let ConnectInfo(addr) =
                ConnectInfo::<SocketAddr>::from_request_parts(&mut parts, &state)
                    .await
                    .map_err(Into::into)?;

            Ok(todo!())
        };
    }
}

#[derive(Debug)]
struct LimiterInfo {
    permits: AtomicU64,
    valid_until: Instant,
}

#[derive(Debug)]
struct Limiter {
    info: RwLock<LimiterInfo>,
    limit: LimitInfo,
}

impl LimiterInfo {
    pub fn new(rate: Rate) -> Self {
        return Self {
            permits: AtomicU64::new(rate.num),
            valid_until: Instant::now() + rate.interval,
        };
    }
}

impl Limiter {
    pub async fn request(self: Arc<Self>) -> Result<()> {
        let mut info = self.info.read().await;
        if info.valid_until.elapsed() >= self.limit.rate.interval {
            drop(info);
            let mut write_info = self.info.write().await;
            *write_info = LimiterInfo::new(self.limit.rate);
            info = write_info.downgrade();
        }

        loop {
            match info.permits.fetch_sub(1, Ordering::Acquire) {
                x if x >= MAX_NUM => {
                    let _ = info.permits.compare_exchange(
                        x,
                        MAX_NUM,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    );

                    match &self.limit.handler {
                        LimitHandler::Wait => {
                            let sleep = tokio::time::sleep_until(info.valid_until);
                            drop(info);
                            sleep.await;
                            info = self.info.read().await
                        }
                        LimitHandler::Fail => return Err(Error::msg("Rate limit exceeded")),
                    }
                }
                _ => break,
            }
        }

        return Ok(());
    }
}

#[pin_project]
pub struct RateLimiterFuture<Fut> {
    #[pin]
    limit: Fuse<BoxFuture<'static, Result<()>>>,
    #[pin]
    fut: Fut,
}

impl<T, E, Fut: Future<Output = Result<T, E>>> Future for RateLimiterFuture<Fut>
where
    Error: Into<E>,
{
    type Output = Result<T, E>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();

        if !this.limit.is_terminated() {
            if this.limit.poll(cx).map_err(Into::into)?.is_pending() {
                return Poll::Pending;
            }
        }

        return this.fut.poll(cx);
    }
}
