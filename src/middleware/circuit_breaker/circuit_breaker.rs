use futures::ready;
use hyper::{Body, Request, Response, header::HeaderName, http::HeaderValue};
use std::time::SystemTime;
use tower::Service;
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use pin_project::pin_project;
use super::state::*;


pub struct CircuitBreakerService<S> {
    inner: S,
    state: Arc<Mutex<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
}


impl<S> CircuitBreakerService<S> {
    pub fn new(inner: S, config: CircuitBreakerConfig) -> Self {
        let state = CircuitBreakerState::Close(CloseState {
            errors: 0,
            last_error: SystemTime::now(),
        });
        CircuitBreakerService { inner, config, state: Arc::new(Mutex::new(state)) }
    }
}


impl<S> Service<Request<Body>> for CircuitBreakerService<S> 
    where S: Service<Request<Body>, Response=Response<Body>, Error=anyhow::Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = CBFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Poll::Ready(r) = self.inner.poll_ready(cx) {
            if self.config.error_threshold > 0 {
                let mut stat = self.state.lock().unwrap();
                if stat.check_state(&self.config) {
                    return Poll::Ready(r)
                }
            } else {  // circurt breaker is off
                return Poll::Ready(r)
            }
        }
        Poll::Pending
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let fut = self.inner.call(req);
        let state = self.state.clone();
        CBFuture { fut, state, config: self.config.clone() }
    }
}


#[pin_project]
pub struct CBFuture<Fut> 
    where Fut: Future<Output=Result<Response<Body>, anyhow::Error>>
{
    #[pin]
    fut: Fut,
    state: Arc<Mutex<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
}


impl<Fut> Future for CBFuture<Fut> 
    where Fut: Future<Output=Result<Response<Body>, anyhow::Error>>
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result: Result<Response<Body>, anyhow::Error> = ready!(this.fut.poll(cx));
        if this.config.error_threshold == 0 {  // circurt breaker is off
            return Poll::Ready(result);
        }
        if let Ok(mut r) = result {
            if r.status().as_u16() >= 500 {
                let mut state = this.state.lock().unwrap();
                state.error(&this.config);
                let header = r.headers_mut();
                let state_value = format!("{:?}", state);
                let state_value = HeaderValue::from_str(&state_value).unwrap();
                header.insert(HeaderName::from_static("circuit-breaker"), state_value);
                Poll::Ready(Ok(r))
            } else {
                let mut state = this.state.lock().unwrap();
                state.success(&this.config);
                let header = r.headers_mut();
                let state_value = format!("{:?}", state);
                let state_value = HeaderValue::from_str(&state_value).unwrap();
                header.insert(HeaderName::from_static("circuit-breaker"), state_value);
                Poll::Ready(Ok(r))
            }
        } else {
            let mut state = this.state.lock().unwrap();
            state.error(&this.config);
            Poll::Ready(result)
        }
    }
}