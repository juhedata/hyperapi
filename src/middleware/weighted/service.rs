use tower::discover::{Change, Discover};
use tower::load::Load;
use tower::ready_cache::{error::Failed, ReadyCache};
use futures_util::ready;
use futures_util::future::{self, TryFutureExt};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::hash::Hash;
use std::marker::PhantomData;
use std::{pin::Pin, task::{Context, Poll}};
use tower::Service;
use tracing::{debug, trace};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Random load balance, use load as service weight
pub struct WeightedBalance<D, Req>
where
    D: Discover,
    D::Key: Hash,
{
    discover: D,
    services: ReadyCache<D::Key, D::Service, Req>,
    ready_index: Option<usize>,
    rng: SmallRng,
    _req: PhantomData<Req>,
}


impl<D, Req> WeightedBalance<D, Req>
where
    D: Discover,
    D::Key: Hash,
    D::Service: Service<Req>,
    <D::Service as Service<Req>>::Error: Into<BoxError>,
{
    /// Constructs a load balancer that uses operating system entropy.
    pub fn new(discover: D) -> Self {
        Self::from_rng(discover, &mut rand::thread_rng()).expect("ThreadRNG must be valid")
    }

    /// Constructs a load balancer seeded with the provided random number generator.
    pub fn from_rng<R: Rng>(discover: D, rng: R) -> Result<Self, rand::Error> {
        let rng = SmallRng::from_rng(rng)?;
        Ok(Self {
            rng,
            discover,
            services: ReadyCache::default(),
            ready_index: None,
            _req: PhantomData,
        })
    }

    /// Returns the number of endpoints currently tracked by the balancer.
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Returns whether or not the balancer is empty.
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }
}

impl<D, Req> WeightedBalance<D, Req>
where
    D: Discover + Unpin,
    D::Key: Hash + Clone,
    D::Error: Into<BoxError>,
    D::Service: Service<Req> + Load,
    <D::Service as Load>::Metric: std::fmt::Debug + Into<u32>,
    <D::Service as Service<Req>>::Error: Into<BoxError>,
{
    fn update_pending_from_discover(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<(), BoxError>>> {
        debug!("updating from discover");
        loop {
            match ready!(Pin::new(&mut self.discover).poll_discover(cx))
                .transpose()
                .map_err(|e| e.into())?
            {
                None => return Poll::Ready(None),
                Some(Change::Remove(key)) => {
                    trace!("remove");
                    self.services.evict(&key);
                }
                Some(Change::Insert(key, svc)) => {
                    trace!("insert");
                    self.services.push(key, svc);
                }
            }
        }
    }

    fn promote_pending_to_ready(&mut self, cx: &mut Context<'_>) {
        loop {
            match self.services.poll_pending(cx) {
                Poll::Ready(Ok(())) => {
                    // There are no remaining pending services.
                    debug_assert_eq!(self.services.pending_len(), 0);
                    break;
                }
                Poll::Pending => {
                    // None of the pending services are ready.
                    debug_assert!(self.services.pending_len() > 0);
                    break;
                }
                Poll::Ready(Err(error)) => {
                    // An individual service was lost; continue processing
                    // pending services.
                    debug!(%error, "dropping failed endpoint");
                }
            }
        }
        trace!(
            ready = %self.services.ready_len(),
            pending = %self.services.pending_len(),
            "poll_unready"
        );
    }

    fn random_ready_index(&mut self) -> Option<usize> {
        match self.services.ready_len() {
            0 => None,
            1 => Some(0),
            len => {
                let mut weights: Vec<u32> = Vec::new();
                for i in 0..len {
                    let (_, svc) = self.services.get_ready_index(i).expect("invalid index");
                    weights.push(svc.load().into())
                }
                let total: u32 = weights.iter().sum();
                let mut point = self.rng.gen_range(0..total);
                for i in 0..weights.len() {
                    point = point - weights[i];
                    if point <= 0 {
                        return Some(i)
                    }
                }
                Some(len - 1)
            }
        }
    }

}

impl<D, Req> Service<Req> for WeightedBalance<D, Req>
where
    D: Discover + Unpin,
    D::Key: Hash + Clone,
    D::Error: Into<BoxError>,
    D::Service: Service<Req> + Load,
    <D::Service as Load>::Metric: std::fmt::Debug + Into<u32>,
    <D::Service as Service<Req>>::Error: Into<BoxError>,
{
    type Response = <D::Service as Service<Req>>::Response;
    type Error = BoxError;
    type Future = future::MapErr<
        <D::Service as Service<Req>>::Future,
        fn(<D::Service as Service<Req>>::Error) -> BoxError,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let _ = self.update_pending_from_discover(cx)?;
        self.promote_pending_to_ready(cx);

        loop {
            if let Some(index) = self.ready_index.take() {
                match self.services.check_ready_index(cx, index) {
                    Ok(true) => {
                        // The service remains ready.
                        self.ready_index = Some(index);
                        return Poll::Ready(Ok(()));
                    }
                    Ok(false) => {
                        // The service is no longer ready. Try to find a new one.
                        trace!("ready service became unavailable");
                    }
                    Err(Failed(_, error)) => {
                        // The ready endpoint failed, so log the error and try
                        // to find a new one.
                        debug!(%error, "endpoint failed");
                    }
                }
            }

            self.ready_index = self.random_ready_index();
            if self.ready_index.is_none() {
                debug_assert_eq!(self.services.ready_len(), 0);
                return Poll::Pending;
            }
        }
    }

    fn call(&mut self, request: Req) -> Self::Future {
        let index = self.ready_index.take().expect("called before ready");
        self.services
            .call_ready_index(index, request)
            .map_err(Into::into)
    }
}
