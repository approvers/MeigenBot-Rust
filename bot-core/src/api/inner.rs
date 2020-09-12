use crate::db::MeigenDatabase;
use percent_encoding::percent_decode;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::reply::Reply;
use warp::reply::{self, json, Json};
use warp::{path, Filter, Rejection};

#[derive(Debug)]
struct RejectionDBError<E: std::fmt::Debug + Send + Sync + 'static>(E);
impl<E: std::fmt::Debug + Send + Sync + 'static> warp::reject::Reject for RejectionDBError<E> {}

#[derive(Debug)]
struct URLDecodeFailed;
impl warp::reject::Reject for URLDecodeFailed {}

type Synced<D> = Arc<RwLock<D>>;

#[derive(Clone)]
pub struct ApiServer<D: MeigenDatabase> {
    address: SocketAddr,
    db: Synced<D>,
}

impl<D: MeigenDatabase> ApiServer<D> {
    pub fn new(address: SocketAddr, db: Synced<D>) -> Self {
        Self { address, db }
    }

    fn with_db(&self) -> impl Filter<Extract = (Synced<D>,), Error = Infallible> + Clone {
        let db = Arc::clone(&self.db);
        warp::any().map(move || Arc::clone(&db))
    }

    pub async fn start(self) {
        log::info!("Starting Meigen Api server at {:?}", self.address);
        let all = path!("all").and(self.with_db()).and_then(Self::all);

        let by_author = path!("author" / String)
            .and(self.with_db())
            .and_then(Self::by_author);

        let paths = all.or(by_author);

        let routes = warp::get()
            .and(paths)
            .recover(Self::recover)
            .with(warp::log("api"));

        warp::serve(routes).run(self.address).await;
    }

    async fn all(db: Synced<D>) -> Result<Json, Rejection> {
        let meigens = db
            .read()
            .await
            .get_all_meigen()
            .await
            .map_err(RejectionDBError)
            .map_err(warp::reject::custom)?;

        Ok(json(&meigens))
    }

    async fn by_author(param: String, db: Synced<D>) -> Result<Json, Rejection> {
        let filter_author = percent_decode(&param.as_bytes())
            .decode_utf8()
            .map_err(|_| URLDecodeFailed)
            .map_err(warp::reject::custom)?;

        let meigens = db
            .read()
            .await
            .search_by_author(&filter_author)
            .await
            .map_err(RejectionDBError)
            .map_err(warp::reject::custom)?;

        Ok(json(&meigens))
    }

    async fn recover(rejection: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(e) = rejection.find::<RejectionDBError<D::Error>>() {
            log::error!("Database connection Error: {:?}", e);

            const TEXT: &str = "Internal Server Error (Failed to communicate with Database)";
            Ok(reply::with_status(TEXT, StatusCode::INTERNAL_SERVER_ERROR))
        //
        } else if let Some(URLDecodeFailed) = rejection.find() {
            const TEXT: &str = "Failed to decode URL";
            Ok(reply::with_status(TEXT, StatusCode::BAD_REQUEST))
        //
        } else {
            Err(rejection)
        }
    }
}
