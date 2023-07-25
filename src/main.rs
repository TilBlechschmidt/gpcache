use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use poem::{
    get, handler,
    listener::TcpListener,
    middleware::AddData,
    web::{Data, Path},
    EndpointExt, Response, Route, Server,
};
use reqwest::StatusCode;

const MAX_AGE: Duration = Duration::from_secs(60 * 60 * 4);
const URL_AUTH: &str = "https://www.space-track.org/ajaxauth/login";
const URL_QUERY: &str = "https://www.space-track.org/basicspacedata/query/class/gp/NORAD_CAT_ID/";

type NoradId = String;

#[derive(Clone)]
struct PerturbationCache(Arc<Mutex<HashMap<NoradId, (Instant, String)>>>);

impl PerturbationCache {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    async fn get_or_fetch(&self, id: NoradId) -> Result<String, Box<dyn std::error::Error>> {
        let cache_entry = self
            .0
            .lock()
            .expect("cache mutex poisoned")
            .get(&id)
            .cloned();

        match cache_entry {
            Some((fetch_time, data)) if fetch_time.elapsed() < MAX_AGE => Ok(data.clone()),
            _ => {
                let data = self.fetch(&id).await?;

                self.0
                    .lock()
                    .expect("cache mutex poisoned")
                    .insert(id, (Instant::now(), data.clone()));

                Ok(data.clone())
            }
        }
    }

    async fn fetch(&self, id: &NoradId) -> Result<String, reqwest::Error> {
        let user =
            std::env::var("SPACETRACK_USER").expect("missing user (env var SPACETRACK_USER)");
        let pass =
            std::env::var("SPACETRACK_PASS").expect("missing password (env var SPACETRACK_PASS)");

        let query = format!("{URL_QUERY}{id}");
        let params = [("identity", user), ("password", pass), ("query", query)];

        let client = reqwest::Client::new();

        client
            .post(URL_AUTH)
            .form(&params)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    }
}

#[handler]
async fn current(Path(id): Path<String>, cache: Data<&PerturbationCache>) -> Response {
    match cache.get_or_fetch(id).await {
        Ok(data) => Response::builder().status(StatusCode::OK).body(data),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(e.to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let cache = PerturbationCache::new();

    let app = Route::new()
        .at("/current/:id", get(current))
        .with(AddData::new(cache));

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
