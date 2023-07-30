use api::SpaceTrackClient;
use perturbation::PerturbationCache;
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{AddData, Cors},
    web::{Data, Json, Path, Query},
    EndpointExt, Response, Route, Server,
};
use reqwest::{Method, StatusCode};
use satellites::{ObjectType, Satellite, SatelliteDatabase};
use serde::Deserialize;
use std::sync::Arc;

mod api;
mod perturbation;
mod satellites;

type NoradId = usize;

const DEFAULT_OBJECT_TYPES: &[ObjectType] = &[
    ObjectType::Payload,
    ObjectType::RocketBody,
    ObjectType::Unknown,
];

#[derive(Deserialize, Debug)]
struct SearchQuery {
    q: String,
}

#[handler]
async fn current(Path(id): Path<usize>, cache: Data<&PerturbationCache>) -> Response {
    match cache.get_or_fetch(id).await {
        Ok(data) => Response::builder().status(StatusCode::OK).body(data),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(e.to_string()),
    }
}

#[handler]
async fn search(q: Query<SearchQuery>, db: Data<&SatelliteDatabase>) -> Json<Vec<Satellite>> {
    Json(db.search(&q.q, DEFAULT_OBJECT_TYPES))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(SpaceTrackClient::from_env());
    let cache = PerturbationCache::new(client.clone());
    let db = SatelliteDatabase::new(client);
    let cors = Cors::new().allow_methods([Method::GET, Method::OPTIONS]);

    // TODO Run this on a timer or smth
    db.update().await?;

    let app = Route::new()
        .at("/search", get(search))
        .at("/current/:id", get(current))
        .with(AddData::new(cache))
        .with(AddData::new(db))
        .with(cors);

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await?;

    Ok(())
}
