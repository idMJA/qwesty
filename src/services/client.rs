use crate::models::{ClientError, Quest, QuestsResponse};
use crate::utils::USER_AGENT;
use log::{debug, info};

pub struct QuestClient {
    client: reqwest::Client,
    super_properties: String,
}

impl QuestClient {
    #[must_use]
    pub fn new(super_properties: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            super_properties,
        }
    }

    pub async fn fetch_quests_with_locale(
        &self,
        token: &str,
        locale: &str,
    ) -> Result<Vec<Quest>, ClientError> {
        debug!("fetching quests via direct API for locale: {}", locale);

        let response = self
            .client
            .get("https://discord.com/api/v10/quests/@me")
            .header("Authorization", token)
            .header("User-Agent", USER_AGENT)
            .header("X-Discord-Locale", locale)
            .header("X-Super-Properties", &self.super_properties)
            .send()
            .await
            .map_err(ClientError::RequestFailed)?;

        if !response.status().is_success() {
            return Err(ClientError::HttpError(response.status().as_u16()));
        }

        let data: QuestsResponse = response.json().await.map_err(ClientError::RequestFailed)?;

        info!(
            "fetched {} quests from API (locale: {})",
            data.quests.len(),
            locale
        );
        Ok(data.quests)
    }
}
