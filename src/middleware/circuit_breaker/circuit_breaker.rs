use futures::ready;
use hyper::{Request, Response, Body};
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
    where S: Service<Request<Body>, Response=Response<Body>, Error=hyper::Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = CBFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Poll::Ready(r) = self.inner.poll_ready(cx) {
            let mut stat = self.state.lock().unwrap();
            if stat.check_state(&self.config) {
                return Poll::Ready(r)
            }
        }
        Poll::Pending
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let f = self.inner.call(req);
        let state = self.state.clone();
        CBFuture {f, state}
    }
}



#[pin_project]
pub struct CBFuture<Fut> {
    #[pin]
    f: Fut,
    state: Arc<Mutex<CircuitBreakerState>>,
}


impl<Fut> Future for CBFuture<Fut> 
    where Fut: Future
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = ready!(this.f.poll(cx));
        if let Ok(r) = result {
            let state = self.state.lock().unwrap();
            state.success(self.config);
        } else {
            let state = self.state.lock().unwrap();
            state.error(self.config);
        }
        Poll::Ready(result)
    }
}