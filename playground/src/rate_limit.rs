use crate::{Error, Result};
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::Request;
use futures::Future;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tower::{Layer, Service};

pub const MAX_NUM: u64 = u64::MAX >> 1;

#[derive(Debug, Clone, Copy)]
pub struct LimitInfo {
    rate: Rate,
    handler: LimitHandler,
}

impl LimitInfo {
    pub fn new(num: u64, interval: Duration, handler: LimitHandler) -> Self {
        return Self {
            rate: Rate { num, interval },
            handler,
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    num: u64,
    interval: Duration,
}

#[derive(Debug, Clone, Copy)]
pub enum LimitHandler {
    Wait,
    Fail,
}

pub struct RateLimit {
    global_info: LimitInfo,
    specific_info: LimitInfo,
}

impl RateLimit {
    pub fn new(global_info: LimitInfo, specific_info: LimitInfo) -> Self {
        return Self {
            global_info,
            specific_info,
        };
    }
}

impl<S> Layer<S> for RateLimit {
    type Service = RateLimitService<S>;

    #[inline]
    fn layer(&self, state: S) -> Self::Service {
        return RateLimitService {
            state,
            specific_info: self.specific_info,
            specific: Arc::new(RwLock::new(HashMap::new())),
        };
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitService<S> {
    state: S,
    // TODO global
    specific_info: LimitInfo,
    specific: Arc<RwLock<HashMap<SocketAddr, Limiter>>>,
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
        let mut state = self.state.clone();
        let specific = self.specific.clone();
        let specific_info = self.specific_info;

        return async move {
            let (mut parts, body) = req.into_parts();
            let ConnectInfo(addr) =
                ConnectInfo::<SocketAddr>::from_request_parts(&mut parts, &state)
                    .await
                    .map_err(Into::into)?;

            // TODO global limiter

            // Specific (by user) limiter
            let read_specific = specific.read().await;
            if let Some(limiter) = read_specific.get(&addr) {
                limiter.request().await.map_err(Into::into)?;
            } else {
                drop(read_specific);
                let mut write_specific = specific.write().await;
                match write_specific.entry(addr) {
                    Entry::Occupied(entry) => entry.get().request().await.map_err(Into::into)?,
                    Entry::Vacant(entry) => {
                        let _ = entry.insert(Limiter::new(specific_info));
                    }
                };
            }

            let req = Request::from_parts(parts, body);
            return state.call(req).await;
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

impl Limiter {
    pub fn new(info: LimitInfo) -> Self {
        return Self {
            info: RwLock::new(LimiterInfo::new(info.rate)),
            limit: info,
        };
    }
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
    pub async fn request(&self) -> Result<()> {
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
