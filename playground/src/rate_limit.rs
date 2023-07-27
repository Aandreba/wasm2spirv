use crate::{Error, Result};
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use futures::Future;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Exclusive};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tower::{Layer, Service};

const CLEANING_INTERVAL: Duration = Duration::from_secs(3600);
const INACTIVITY_THRESHOLD: Duration = Duration::from_secs(600);

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

#[derive(Debug, Clone)]
pub struct RateLimit {
    global_info: Option<LimitInfo>,
    specific_info: Option<LimitInfo>,
}

impl RateLimit {
    pub fn new(
        global_info: impl Into<Option<LimitInfo>>,
        specific_info: impl Into<Option<LimitInfo>>,
    ) -> Self {
        return Self {
            global_info: global_info.into(),
            specific_info: specific_info.into(),
        };
    }
}

impl<S> Layer<S> for RateLimit {
    type Service = RateLimitService<S>;

    #[inline]
    fn layer(&self, state: S) -> Self::Service {
        let specific = self.specific_info.map(|info| {
            (
                info,
                Arc::new(RwLock::new(HashMap::<SocketAddr, Limiter>::new())),
            )
        });

        // Periodically clean up unused limiters
        let mut cleaner_killer = None;
        if let Some((_, specific)) = specific.clone() {
            let (flag, sub) = utils_atomics::flag::mpsc::async_flag();
            cleaner_killer = Some(flag);

            let cleaner = async move {
                let mut iter = tokio::time::interval(CLEANING_INTERVAL);
                loop {
                    let _ = iter.tick().await;
                    let mut specific = specific.write().await;

                    let mut keys_to_delete = Vec::with_capacity(specific.len());
                    for (key, value) in specific.iter_mut() {
                        let state = value.state.get_mut();
                        if state.valid_until.elapsed() >= INACTIVITY_THRESHOLD {
                            keys_to_delete.push(*key);
                        }
                    }

                    for key in keys_to_delete {
                        specific.remove(&key);
                    }
                }
            };

            tokio::spawn(futures::future::select(sub, Box::pin(cleaner)));
        }

        return RateLimitService {
            state,
            global: self.global_info.map(|info| Arc::new(Limiter::new(info))),
            specific,
            _cleaner_killer: cleaner_killer,
        };
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitService<S> {
    state: S,
    global: Option<Arc<Limiter>>,
    specific: Option<(LimitInfo, Arc<RwLock<HashMap<SocketAddr, Limiter>>>)>,
    _cleaner_killer: Option<utils_atomics::flag::mpsc::AsyncFlag>,
}

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    B: 'static + Send,
    S: 'static + Service<Request<B>> + Clone + Send,
    S::Response: 'static + IntoResponse,
    S::Error: 'static + Into<Infallible>,
    S::Future: 'static + Send,
    <ConnectInfo<SocketAddr> as FromRequestParts<Exclusive<S>>>::Rejection: IntoResponse,
{
    type Response = Response;
    type Error = S::Error;
    type Future = impl 'static + Future<Output = Result<Self::Response, Self::Error>> + Send;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.state.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        macro_rules! tri {
            ($e:expr) => {
                match $e {
                    Ok(x) => x,
                    Err(e) => return Ok(e.into_response()),
                }
            };
        }

        let mut state = Exclusive::new(self.state.clone());
        let global = self.global.clone();
        let specific = self.specific.clone();

        let fut = async move {
            let (mut parts, body) = req.into_parts();

            // TODO global limiter
            if let Some(global) = global {
                tri!(global.request().await);
            }

            // Specific (by user) limiter
            if let Some((specific_info, specific)) = specific {
                let ConnectInfo(addr) =
                    tri!(ConnectInfo::<SocketAddr>::from_request_parts(&mut parts, &state).await);

                let read_specific = specific.read().await;
                if let Some(limiter) = read_specific.get(&addr) {
                    tri!(limiter.request().await);
                } else {
                    drop(read_specific);
                    let mut write_specific = specific.write().await;
                    match write_specific.entry(addr) {
                        Entry::Occupied(entry) => tri!(entry.get().request().await),
                        Entry::Vacant(entry) => {
                            let _ = entry.insert(Limiter::new(specific_info));
                        }
                    };
                }
            }

            let req = Request::from_parts(parts, body);
            return state
                .get_mut()
                .call(req)
                .await
                .map(IntoResponse::into_response);
        };

        return fut;
    }
}

#[derive(Debug)]
struct Limiter {
    state: RwLock<LimiterState>,
    info: LimitInfo,
}

impl Limiter {
    pub fn new(info: LimitInfo) -> Self {
        return Self {
            state: RwLock::new(LimiterState::new(info.rate)),
            info,
        };
    }
}

impl Limiter {
    pub async fn request(&self) -> Result<()> {
        let mut info = self.state.read().await;

        loop {
            if Instant::now() >= info.valid_until {
                drop(info);
                let mut write_info = self.state.write().await;
                *write_info = LimiterState::new(self.info.rate);
                info = write_info.downgrade();
            }

            match info.permits.fetch_sub(1, Ordering::AcqRel) {
                x if (x - 1).is_negative() => {
                    let _ = info.permits.compare_exchange(
                        x - 1,
                        -1,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    );

                    match &self.info.handler {
                        LimitHandler::Wait => {
                            let sleep = tokio::time::sleep_until(info.valid_until);
                            drop(info);
                            sleep.await;
                            info = self.state.read().await
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

#[derive(Debug)]
struct LimiterState {
    permits: AtomicI64,
    valid_until: Instant,
}

impl LimiterState {
    pub fn new(rate: Rate) -> Self {
        return Self {
            permits: AtomicI64::new(rate.num as i64),
            valid_until: Instant::now() + rate.interval,
        };
    }
}
