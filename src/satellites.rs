use crate::{api::SpaceTrackClient, NoradId};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    cmp::Reverse,
    collections::HashMap,
    fmt::Display,
    str::FromStr,
    sync::{Arc, RwLock},
};
use sublime_fuzzy::best_match;

const QUERY_URL: &str = "https://www.space-track.org/basicspacedata/query/class/satcat/orderby/NORAD_CAT_ID%20asc/emptyresult/show";

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub enum ObjectType {
    RocketBody,
    Payload,
    Debris,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct OrbitData {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    period: f64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    inclination: f64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    apogee: f64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    perigee: f64,
}

// Unused fields are commented out but do exist if needed in the future
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Satellite {
    #[serde(
        rename = "NORAD_CAT_ID",
        deserialize_with = "deserialize_number_from_string"
    )]
    id: NoradId,

    // object_id: String,
    object_type: ObjectType,
    object_name: String,

    // country: String,
    // site: String,
    launch: String, // 1957-10-04
    decay: Option<String>,

    #[serde(flatten)]
    orbit: Option<OrbitData>,
}

#[derive(Serialize, Debug)]
pub struct SearchResult {
    score: isize,
    satellite: Satellite,
}

#[derive(Clone)]
pub struct SatelliteDatabase {
    client: Arc<SpaceTrackClient>,
    entries: Arc<RwLock<HashMap<NoradId, Satellite>>>,
}

impl SatelliteDatabase {
    pub fn new(client: Arc<SpaceTrackClient>) -> Self {
        Self {
            client,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Updating satellite database ...");

        let satellites = self.fetch().await?;

        println!("Ingesting satellite list ...");

        let mut entries = self.entries.write().expect("satellite mutex poisoned");
        *entries = satellites.into_iter().map(|s| (s.id.clone(), s)).collect();

        println!("Updated satellite database with {} entries", entries.len());

        Ok(())
    }

    pub fn search(&self, query: &str, allowed_types: &[ObjectType]) -> Vec<Satellite> {
        // Protect our CPU :3
        if query.len() <= 3 {
            return Vec::new();
        }

        let entries = self.entries.read().expect("satellite mutex poisoned");

        // Try to short-circuit if the query is likely to be an ID and we have a matching entry
        if let Some(satellite) = query
            .parse::<usize>()
            .ok()
            .map(|id| entries.get(&id))
            .flatten()
            .cloned()
        {
            return vec![satellite];
        }

        // Fall back to fuzzy search
        let mut matches = entries
            .values()
            .filter(|s| allowed_types.contains(&s.object_type))
            .filter_map(|s| {
                if let Some(m) = best_match(query, &s.object_name) {
                    let score = m.score();

                    // Reject really bad results
                    if score < 0 {
                        return None;
                    }

                    Some(SearchResult {
                        score,
                        satellite: s.clone(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Sort by ID first and then stably by the score
        // which puts more recent sats at the top if the score is equal.
        matches.sort_unstable_by_key(|r| Reverse(r.satellite.id));
        matches.sort_by_key(|r| Reverse(r.score));

        // Only return the top-n results
        matches.truncate(20);

        matches.into_iter().map(|r| r.satellite).collect()
    }

    async fn fetch(&self) -> Result<Vec<Satellite>, reqwest::Error> {
        self.client.query(QUERY_URL.into()).await?.json().await
    }
}

fn deserialize_number_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt<T> {
        String(String),
        Number(T),
    }

    match StringOrInt::<T>::deserialize(deserializer)? {
        StringOrInt::String(s) => s.parse::<T>().map_err(serde::de::Error::custom),
        StringOrInt::Number(i) => Ok(i),
    }
}

impl<'de> Deserialize<'de> for ObjectType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        match string.as_str() {
            "ROCKET BODY" => Ok(ObjectType::RocketBody),
            "PAYLOAD" => Ok(ObjectType::Payload),
            "DEBRIS" => Ok(ObjectType::Debris),
            "UNKNOWN" => Ok(ObjectType::Unknown),
            _ => {
                eprintln!(
                    "Encountered unknown object type '{}', mapping to Unknown",
                    string
                );
                Ok(ObjectType::Unknown)
            }
        }
    }
}
