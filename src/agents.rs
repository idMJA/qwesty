use crate::models::Config;
use crate::services::QuestClient;
use log::info;

pub async fn agent_cycle(
    client: &QuestClient,
    config: &Config,
    locales: &[String],
) -> Result<(), String> {
    let locale = locales.first().map_or("en-US", String::as_str);
    info!("agent fetching locale {locale}");
    let quests = client
        .fetch_quests_with_locale(&config.discord.token, locale)
        .await
        .map_err(|e| format!("failed to fetch quests for agent {locale}: {e}"))?;

    let Some(url) = config.collector_url() else {
        return Err("collector_url not configured".to_string());
    };
    let Some(token) = config.collector_token() else {
        return Err("collector_token not configured".to_string());
    };

    let payload = serde_json::json!({
        "region": locale,
        "quests": quests,
        "source": "agent"
    });

    let client_http = reqwest::Client::new();
    let res = client_http
        .post(url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("agent failed to POST to collector: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("collector responded with status {}", res.status()));
    }

    info!("agent payload sent successfully to {url}");
    Ok(())
}
