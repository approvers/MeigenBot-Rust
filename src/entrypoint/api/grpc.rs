use std::{convert::TryInto, net::SocketAddr, sync::Arc};

use anyhow::Context as _;
use async_trait::async_trait;
use protobuf::{
    meigen_api_server::{MeigenApi, MeigenApiServer},
    GetRequest, GetResponse, RandomRequest, RandomResponse, SearchRequest, SearchResponse,
};
use tokio::sync::RwLock;
use tonic::{transport::Server, Code, Request, Response, Status};

use super::{
    auth::{self, Authenticator},
    CustomError,
};
use crate::{db::MeigenDatabase, Synced};

mod protobuf {
    tonic::include_proto!("meigen_api");

    impl From<crate::model::Meigen> for Meigen {
        fn from(v: crate::model::Meigen) -> Self {
            Self {
                id: v.id,
                author: v.author,
                content: v.content,
            }
        }
    }
}

pub struct GrpcServer<A, D> {
    auth: A,
    db: Synced<D>,
}

impl<A, D> GrpcServer<A, D>
where
    A: Authenticator,
    D: MeigenDatabase,
{
    pub fn new(db: D, auth: A) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            auth,
        }
    }

    pub async fn start(self, ip: impl Into<SocketAddr>) -> anyhow::Result<()> {
        let ip = ip.into();
        tracing::info!("starting grpc server at {}", ip);

        Server::builder()
            .add_service(MeigenApiServer::new(self))
            .serve(ip)
            .await
            .context("failed to start server")
    }

    async fn auth<T>(&self, request: &tonic::Request<T>) -> Result<(), Status> {
        let token = request
            .metadata()
            .get("gauth-token")
            .ok_or_else(|| Status::unauthenticated("gauth-token metadata is missing"))?;

        let token_str = match token.to_str() {
            Ok(s) => s,
            Err(_) => return Err(Status::unauthenticated("failed to decode gauth-token")),
        };

        match self.auth.auth(token_str).await {
            Ok(_) => Ok(()),

            Err(auth::Error::Internal(e)) => {
                tracing::error!("internal error: {:#?}", e);
                Err(Status::internal("internal server error"))
            }

            Err(auth::Error::InvalidToken) => Err(Status::unauthenticated("invalid token")),
        }
    }
}

#[async_trait]
impl<A, D> MeigenApi for GrpcServer<A, D>
where
    A: Authenticator,
    D: MeigenDatabase,
{
    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        self.auth(&request).await?;

        let result = super::get(request.into_inner().id, Arc::clone(&self.db))
            .await
            .map_err(into_status)?;

        Ok(Response::new(GetResponse {
            meigen: result.map(From::from),
        }))
    }

    async fn random(
        &self,
        request: Request<RandomRequest>,
    ) -> Result<Response<RandomResponse>, Status> {
        self.auth(&request).await?;

        let request = request.into_inner();
        let request = super::RandomRequest {
            count: request.count.map(|x| x as _),
        };

        let result = super::random(request, Arc::clone(&self.db))
            .await
            .map_err(into_status)?
            .into_iter()
            .map(From::from)
            .collect();

        Ok(Response::new(RandomResponse { meigen: result }))
    }

    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        self.auth(&request).await?;

        let request = request.into_inner();

        let limit: Option<u8> = match request.limit {
            Some(t) => Some(
                t.try_into()
                    .map_err(|_| Status::invalid_argument("limit option is too big"))?,
            ),
            None => None,
        };

        let request = super::SearchRequest {
            limit,
            offset: request.offset,
            author: request.author,
            content: request.content,
        };

        let result = super::search(request, Arc::clone(&self.db))
            .await
            .map_err(into_status)?
            .into_iter()
            .map(From::from)
            .collect();

        Ok(Response::new(SearchResponse { meigen: result }))
    }
}

fn into_status(c: CustomError) -> Status {
    let code = match c {
        CustomError::Internal(ref e) => {
            tracing::error!("internal error: {:#?}", e);
            Code::Internal
        }

        CustomError::SearchWordLengthLimitExceeded => Code::InvalidArgument,
        CustomError::FetchLimitExceeded => Code::InvalidArgument,
        CustomError::TooBigOffset => Code::OutOfRange,
        CustomError::Authentication => Code::Unauthenticated,
    };

    Status::new(code, c.describe())
}
