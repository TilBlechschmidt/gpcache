use reqwest::{Response, StatusCode};
use std::time::{Duration, Instant};

const URL_AUTH: &str = "https://www.space-track.org/ajaxauth/login";

pub struct SpaceTrackClient {
    client: reqwest::Client,

    user: String,
    pass: String,

    last_auth: Instant,
}

impl SpaceTrackClient {
    pub async fn from_env() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("HTTP client should build");

        let user =
            std::env::var("SPACETRACK_USER").expect("missing user (env var SPACETRACK_USER)");
        let pass =
            std::env::var("SPACETRACK_PASS").expect("missing password (env var SPACETRACK_PASS)");

        let instance = Self {
            client,
            user,
            pass,
            last_auth: Instant::now(),
        };

        instance.reauth(true).await?;

        Ok(instance)
    }

    async fn reauth(&self, force: bool) -> Result<(), reqwest::Error> {
        if self.last_auth.elapsed() > Duration::from_secs(300) && !force {
            println!("Skipping reauth, too frequent ...");
            return Ok(());
        }

        println!("Auth expired, reauthenticating ...");

        let params = [("identity", &self.user), ("password", &self.pass)];

        self.client
            .post(URL_AUTH)
            .form(&params)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn query(&self, url: String) -> Result<Response, reqwest::Error> {
        let response = self.client.get(&url).send().await?;

        if response.status() != StatusCode::UNAUTHORIZED {
            response.error_for_status()
        } else {
            self.reauth(false).await?;
            self.client.get(&url).send().await?.error_for_status()
        }
    }
}
