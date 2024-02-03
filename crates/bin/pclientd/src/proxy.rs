use futures::FutureExt;
use http_body::Body as _;
use std::convert::Infallible;
use std::pin::Pin;
use std::{
    future::Future,
    task::{Context, Poll},
};
use tonic::transport::NamedService;
use tonic::{
    body::BoxBody,
    transport::{Body, Channel},
};
use tower::ServiceExt;

fn proxy(
    channel: Channel,
    req: http::Request<Body>,
) -> Pin<Box<dyn Future<Output = Result<http::Response<BoxBody>, Infallible>> + Send + 'static>> {
    tracing::debug!(headers = ?req.headers(), "proxying request");
    // Convert request types
    let req = req.map(|b| {
        b.map_err(|e| tonic::Status::from_error(Box::new(e)))
            .boxed_unsync()
    });

    let rsp = channel.oneshot(req);

    async move {
        // Once we get the response, we need to convert any transport errors into
        // an Ok(HTTP response reporting an internal error), so we can have Error = Infallible
        let rsp = match rsp.await {
            Ok(rsp) => rsp.map(|b| {
                b.map_err(|e| tonic::Status::from_error(Box::new(e)))
                    .boxed_unsync()
            }),
            Err(e) => tonic::Status::internal(format!("grpc proxy error: {e}")).to_http(),
        };
        Ok::<_, Infallible>(rsp)
    }
    .boxed()
}

#[derive(Clone)]
pub struct AppQueryProxy(pub Channel);

impl NamedService for AppQueryProxy {
    const NAME: &'static str = "penumbra.core.app.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for AppQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct GovernanceQueryProxy(pub Channel);

impl NamedService for GovernanceQueryProxy {
    const NAME: &'static str = "penumbra.core.component.governance.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for GovernanceQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct DexQueryProxy(pub Channel);

impl NamedService for DexQueryProxy {
    const NAME: &'static str = "penumbra.core.component.dex.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for DexQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct DexSimulationProxy(pub Channel);

impl NamedService for DexSimulationProxy {
    const NAME: &'static str = "penumbra.core.component.dex.v1alpha1.SimulationService";
}

impl tower::Service<http::Request<Body>> for DexSimulationProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct FeeQueryProxy(pub Channel);

impl NamedService for FeeQueryProxy {
    const NAME: &'static str = "penumbra.core.component.fee.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for FeeQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct SctQueryProxy(pub Channel);

impl NamedService for SctQueryProxy {
    const NAME: &'static str = "penumbra.core.component.sct.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for SctQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct ShieldedPoolQueryProxy(pub Channel);

impl NamedService for ShieldedPoolQueryProxy {
    const NAME: &'static str = "penumbra.core.component.shielded_pool.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for ShieldedPoolQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct ChainQueryProxy(pub Channel);

impl NamedService for ChainQueryProxy {
    const NAME: &'static str = "penumbra.core.component.chain.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for ChainQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct StakeQueryProxy(pub Channel);

impl NamedService for StakeQueryProxy {
    const NAME: &'static str = "penumbra.core.component.stake.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for StakeQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct CompactBlockQueryProxy(pub Channel);

impl NamedService for CompactBlockQueryProxy {
    const NAME: &'static str = "penumbra.core.component.compact_block.v1alpha1.QueryService";
}

impl tower::Service<http::Request<Body>> for CompactBlockQueryProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}

#[derive(Clone)]
pub struct TendermintProxyProxy(pub Channel);

impl NamedService for TendermintProxyProxy {
    const NAME: &'static str = "penumbra.util.tendermint_proxy.v1alpha1.TendermintProxyService";
}

impl tower::Service<http::Request<Body>> for TendermintProxyProxy {
    type Response = http::Response<BoxBody>;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Body>) -> Self::Future {
        proxy(self.0.clone(), req)
    }
}
