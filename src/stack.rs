use tower::{Service, layer::Layer};
use std::task::{Context, Poll};


#[derive(Clone, Debug)]
pub struct Stack<S>(S);

impl<S> Stack<S> {

    pub fn new(inner: S) -> Self {
        Self(inner)
    }

    pub fn push<L: Layer<S>>(self, layer: L) -> Stack<L::Service> {
        Stack(layer.layer(self.0))
    }
}


impl<T, S> Service<T> for Stack<S>
where
    S: Service<T>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, t: T) -> Self::Future {
        self.0.call(t)
    }
}
