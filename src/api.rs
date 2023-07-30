use reqwest::Response;

const URL_AUTH: &str = "https://www.space-track.org/ajaxauth/login";

pub struct SpaceTrackClient {
    client: reqwest::Client,
    user: String,
    pass: String,
}

impl SpaceTrackClient {
    pub fn from_env() -> Self {
        let client = reqwest::Client::new();
        let user =
            std::env::var("SPACETRACK_USER").expect("missing user (env var SPACETRACK_USER)");
        let pass =
            std::env::var("SPACETRACK_PASS").expect("missing password (env var SPACETRACK_PASS)");

        Self { client, user, pass }
    }

    pub async fn query(&self, query: String) -> Result<Response, reqwest::Error> {
        let params = [
            ("identity", &self.user),
            ("password", &self.pass),
            ("query", &query),
        ];

        self.client
            .post(URL_AUTH)
            .form(&params)
            .send()
            .await?
            .error_for_status()
    }
}
