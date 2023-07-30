use crate::{api::SpaceTrackClient, NoradId};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const MAX_AGE: Duration = Duration::from_secs(60 * 60 * 4);
const QUERY_URL: &str = "https://www.space-track.org/basicspacedata/query/class/gp/NORAD_CAT_ID";

#[derive(Clone)]
pub struct PerturbationCache {
    entries: Arc<Mutex<HashMap<NoradId, (Instant, String)>>>,
    client: Arc<SpaceTrackClient>,
}

impl PerturbationCache {
    pub fn new(client: Arc<SpaceTrackClient>) -> Self {
        Self {
            client,
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_or_fetch(&self, id: NoradId) -> Result<String, Box<dyn std::error::Error>> {
        let cache_entry = self
            .entries
            .lock()
            .expect("cache mutex poisoned")
            .get(&id)
            .cloned();

        match cache_entry {
            Some((fetch_time, data)) if fetch_time.elapsed() < MAX_AGE => Ok(data.clone()),
            _ => {
                let data = self.fetch(&id).await?;

                self.entries
                    .lock()
                    .expect("cache mutex poisoned")
                    .insert(id, (Instant::now(), data.clone()));

                Ok(data.clone())
            }
        }
    }

    pub async fn fetch(&self, id: &NoradId) -> Result<String, reqwest::Error> {
        self.client
            .query(format!("{QUERY_URL}/{id}"))
            .await?
            .text()
            .await
    }
}
