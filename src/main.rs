mod models;
mod services;
mod utils;

use crate::utils::dedupe_by_key;
use log::*;
use models::{AppError, Config, LOCALES};
use services::{storage, QuestClient};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = Config::load().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    // Log token and first webhook url (if any)
    let first_webhook = config
        .discord
        .webhooks
        .as_ref()
        .and_then(|w| w.first())
        .map(|entry| entry.url.clone());

    debug!(
        "Config loaded - Token: {}..., Webhooks: {}...",
        &config.discord.token[..20.min(config.discord.token.len())],
        &first_webhook.unwrap_or_else(|| "(none)".to_string())
    );

    storage::init_storage(config.storage_type(), config.storage_path());

    info!(
        "starting Discord Quest Notifier - filter={}, interval={} min, run_once={}, storage_type={}",
        config.reward_filter(),
        config.fetch_interval(),
        config.run_once(),
        config.storage_type()
    );

    let client = QuestClient::new(config.super_properties().to_string());

    // support multiple webhooks defined under [discord] as [[discord.webhooks]] = [{ name = "x", url = "..." }, ...]
    let notifiers: Vec<services::webhook::WebhookNotifier> = match &config.discord.webhooks {
        Some(entries) if !entries.is_empty() => entries
            .iter()
            .map(|entry| {
                services::webhook::WebhookNotifier::new(entry.url.clone(), entry.name.clone())
            })
            .collect(),
        _ => {
            return Err(Box::<dyn std::error::Error>::from(AppError(
                "No webhooks configured. Please add [[discord.webhooks]] entries in config.toml"
                    .to_string(),
            )));
        }
    };

    let primary_webhook = config
        .discord
        .webhooks
        .as_ref()
        .and_then(|entries| entries.first().cloned());

    if let Some(primary) = primary_webhook {
        info!(
            "Primary webhook configured: {} {}",
            primary.name.unwrap_or_else(|| "(unnamed)".to_string()),
            primary.url
        );
    }

    let locales_to_check: Vec<&str> = if config.locale_mode() == "all" {
        LOCALES.to_vec()
    } else {
        vec!["en-US"]
    };

    info!(
        "using locale mode: {} (will check {} locale(s))",
        config.locale_mode(),
        locales_to_check.len()
    );

    loop {
        match check_quests_all_locales(
            &client,
            &notifiers,
            &config.discord.token,
            config.reward_filter(),
            &locales_to_check,
            config.initial_send_all(),
        )
        .await
        {
            Ok(_) => {
                if config.run_once() {
                    info!("RUN_ONCE mode: exiting after first check");
                    break;
                }
            }
            Err(e) => {
                error!("error checking quests: {}", e);
                if config.run_once() {
                    return Err(Box::<dyn std::error::Error>::from(AppError(e)));
                }
            }
        }

        if !config.run_once() {
            info!("next check in {} minutes", config.fetch_interval());
            sleep(Duration::from_secs(config.fetch_interval() * 60)).await;
        }
    }

    Ok(())
}

async fn check_quests_all_locales(
    client: &QuestClient,
    notifiers: &[services::webhook::WebhookNotifier],
    token: &str,
    reward_filter: &str,
    locales: &[&str],
    initial_send_all: bool,
) -> Result<(), String> {
    let mut all_filtered_quests = Vec::new();
    let mut stored = storage::load_stored_quests();
    let seed_only = stored.is_empty() && !initial_send_all;

    for (index, locale) in locales.iter().enumerate() {
        if index > 0 {
            let delay_secs: u64 = 60 + (rand::random::<u8>() % 11) as u64; // 60..70
            info!(
                "waiting {} seconds before checking next locale ({})",
                delay_secs, locale
            );
            sleep(Duration::from_secs(delay_secs)).await;
        }

        info!("checking quests for locale: {}", locale);

        let quests = client
            .fetch_quests_with_locale(token, locale)
            .await
            .map_err(|e| format!("failed to fetch quests for locale {}: {}", locale, e))?;

        let filtered = storage::filter_quests(&quests, reward_filter);
        info!(
            "fetched {} quests, filtered to {} (locale: {}, filter={})",
            quests.len(),
            filtered.len(),
            locale,
            reward_filter
        );

        let new_for_locale = if seed_only {
            Vec::new()
        } else {
            storage::find_new_quests(&filtered, &stored)
        };

        if !new_for_locale.is_empty() {
            info!(
                "found {} new quests for locale {}",
                new_for_locale.len(),
                locale
            );

            // convert new stored quests to ids and find the full Quest objects for the locale
            let new_ids: Vec<String> = new_for_locale.iter().map(|q| q.id.clone()).collect();
            let full_new_quests: Vec<_> = quests
                .iter()
                .filter(|q| new_ids.contains(&q.config.id))
                .cloned()
                .collect();

            for notifier in notifiers.iter() {
                notifier.notify_full(&full_new_quests).await.map_err(|e| {
                    format!("failed to send notifications for locale {}: {}", locale, e)
                })?;
            }

            // append newly seen quests to stored and persist so subsequent locales wont re-notify that trash thing again
            stored.extend(new_for_locale.iter().cloned());
            let merged_local_stored = dedupe_by_key(&stored, |q| q.id.clone());
            storage::save_quests(&merged_local_stored)
                .map_err(|e| format!("failed to save quests after locale {}: {}", locale, e))?;
        } else {
            debug!("no new quests for locale {}", locale);
        }

        all_filtered_quests.extend(filtered);
    }

    let stored = storage::load_stored_quests();
    let new_quests = storage::find_new_quests(&all_filtered_quests, &stored);
    info!("found {} new quests across all locales", new_quests.len());

    if !new_quests.is_empty() {
        let all_quests = client
            .fetch_quests_with_locale(token, "en-US")
            .await
            .map_err(|e| format!("failed to fetch quests for notifications: {}", e))?;

        let new_quest_ids: Vec<String> = new_quests.iter().map(|q| q.id.clone()).collect();
        let full_new_quests: Vec<_> = all_quests
            .iter()
            .filter(|q| new_quest_ids.contains(&q.config.id))
            .cloned()
            .collect();

        for notifier in notifiers.iter() {
            notifier
                .notify_full(&full_new_quests)
                .await
                .map_err(|e| format!("failed to send notifications: {}", e))?;
        }
    }

    let merged_quests = dedupe_by_key(&all_filtered_quests, |q| q.id.clone());
    storage::save_quests(&merged_quests).map_err(|e| format!("failed to save quests: {}", e))?;

    Ok(())
}
