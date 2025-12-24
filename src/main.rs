mod agents;
mod collector;
mod models;
mod services;
mod utils;

use log::{debug, error, info};
use models::{AppError, Config, LOCALES};
use services::{storage, QuestClient};
use std::time::Duration;

type AppInit = (
    QuestClient,
    Vec<services::webhook::WebhookNotifier>,
    Vec<String>,
);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = Config::load()
        .map_err(|e| {
            eprintln!("Failed to load configuration: {e}");
            e
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let (client, notifiers, locales_to_check) = init_app(&config)?;

    info!(
        "role={}, using locale mode: {} (will check {} locale(s))",
        config.role(),
        config.locale_mode(),
        locales_to_check.len()
    );

    loop {
        if config.is_agent() {
            match agents::agent_cycle(&client, &config, &locales_to_check).await {
                Ok(()) => {
                    if config.run_once() {
                        break;
                    }
                }
                Err(e) => {
                    error!("agent error: {e}");
                    if config.run_once() {
                        return Err(Box::<dyn std::error::Error>::from(AppError(e)));
                    }
                }
            }
        } else {
            match collector::check_quests_all_locales(
                &client,
                &notifiers,
                &config.discord.token,
                config.reward_filter(),
                &locales_to_check,
                config.initial_send_all(),
            )
            .await
            {
                Ok(()) => {
                    if config.run_once() {
                        info!("RUN_ONCE mode: exiting after first check");
                        break;
                    }
                }
                Err(e) => {
                    error!("error checking quests: {e}");
                    if config.run_once() {
                        return Err(Box::<dyn std::error::Error>::from(AppError(e)));
                    }
                }
            }
        }

        if !config.run_once() {
            info!("next check in {} minutes", config.fetch_interval());
            tokio::time::sleep(Duration::from_secs(config.fetch_interval() * 60)).await;
        }
    }
    Ok(())
}

fn init_app(config: &Config) -> Result<AppInit, Box<dyn std::error::Error>> {
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
                services::webhook::WebhookNotifier::new(
                    entry.url.clone(),
                    entry.name.clone(),
                    entry.message.clone(),
                )
            })
            .collect(),
        _ => {
            if config.is_agent() {
                // agent mode does not require webhooks
                Vec::new()
            } else {
                return Err(Box::<dyn std::error::Error>::from(AppError(
                    "No webhooks configured. Please add [[discord.webhooks]] entries in config.toml"
                        .to_string(),
                )));
            }
        }
    };

    // If collector, start ingest server concurrently
    if config.is_collector() {
        let accept = config.accept_token().map(ToString::to_string);
        let port = config.ingest_port();
        let notifiers_clone = notifiers.clone();
        tokio::spawn(async move {
            services::ingest::start_server(accept, port, notifiers_clone).await;
        });
    }

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

    let locales_to_check: Vec<String> = if config.is_agent() {
        vec![config.region_code().to_string()]
    } else if config.locale_mode() == "all" {
        LOCALES.iter().map(|s| (*s).to_string()).collect()
    } else {
        vec![config.region_code().to_string()]
    };

    Ok((client, notifiers, locales_to_check))
}
