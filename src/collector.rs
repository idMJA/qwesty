use crate::services::{storage, webhook::WebhookNotifier, QuestClient};
use crate::utils::dedupe_by_key;
use log::{debug, info};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;

pub async fn check_quests_all_locales(
    client: &QuestClient,
    notifiers: &[WebhookNotifier],
    token: &str,
    reward_filter: &str,
    locales: &[String],
    initial_send_all: bool,
) -> Result<(), String> {
    let mut all_filtered_quests = Vec::new();
    let mut stored = storage::load_stored_quests();
    let seed_only = stored.is_empty() && !initial_send_all;

    for (index, locale) in locales.iter().enumerate() {
        if index > 0 {
            let delay_secs: u64 = 60 + u64::from(rand::random::<u8>() % 11); // 60..70
            info!("waiting {delay_secs} seconds before checking next locale ({locale})");
            sleep(Duration::from_secs(delay_secs)).await;
        }

        info!("checking quests for locale: {locale}");

        let quests = client
            .fetch_quests_with_locale(token, locale)
            .await
            .map_err(|e| format!("failed to fetch quests for locale {locale}: {e}"))?;

        let filtered = storage::filter_quests(&quests, reward_filter);
        let filtered_len = filtered.len();

        let filtered_prefixed: Vec<_> = filtered
            .into_iter()
            .map(|mut sq| {
                sq.id = format!("{}:{}", locale, sq.id);
                sq
            })
            .collect();

        let seen_base: HashSet<String> = stored
            .iter()
            .map(|q| {
                q.id.split(':')
                    .next_back()
                    .unwrap_or(q.id.as_str())
                    .to_string()
            })
            .collect();

        let mut new_for_locale = if seed_only {
            Vec::new()
        } else {
            storage::find_new_quests(&filtered_prefixed, &stored)
        };

        new_for_locale.retain(|q| {
            let base =
                q.id.split(':')
                    .next_back()
                    .unwrap_or(q.id.as_str())
                    .to_string();
            !seen_base.contains(&base)
        });

        info!(
            "fetched {} quests, filtered to {} (locale: {}, filter={})",
            quests.len(),
            filtered_len,
            locale,
            reward_filter
        );

        if new_for_locale.is_empty() {
            debug!("no new quests for locale {locale}");
        } else {
            info!(
                "found {} new quests for locale {locale}",
                new_for_locale.len()
            );

            let new_ids: Vec<String> = new_for_locale
                .iter()
                .map(|q| {
                    q.id.split(':')
                        .next_back()
                        .unwrap_or(q.id.as_str())
                        .to_string()
                })
                .collect();
            let full_new_quests: Vec<_> = quests
                .iter()
                .filter(|q| new_ids.contains(&q.config.id))
                .cloned()
                .collect();

            for notifier in notifiers {
                notifier.notify_full(&full_new_quests).await.map_err(|e| {
                    format!("failed to send notifications for locale {locale}: {e}")
                })?;
            }

            stored.extend(new_for_locale.iter().cloned());
            let merged_local_stored = dedupe_by_key(&stored, |q| q.id.clone());
            storage::save_quests(&merged_local_stored)
                .map_err(|e| format!("failed to save quests after locale {locale}: {e}"))?;
        }

        all_filtered_quests.extend(filtered_prefixed);
    }

    let stored = storage::load_stored_quests();
    let new_quests = storage::find_new_quests(&all_filtered_quests, &stored);
    info!("found {} new quests across all locales", new_quests.len());

    if seed_only {
        info!("initial run detected and initial_send_all=false; skipping notifications and seeding storage");
    } else if !new_quests.is_empty() {
        let all_quests = client
            .fetch_quests_with_locale(token, "en-US")
            .await
            .map_err(|e| format!("failed to fetch quests for notifications: {e}"))?;

        let new_quest_ids: Vec<String> = new_quests
            .iter()
            .map(|q| {
                q.id.split(':')
                    .next_back()
                    .unwrap_or(q.id.as_str())
                    .to_string()
            })
            .collect();
        let full_new_quests: Vec<_> = all_quests
            .iter()
            .filter(|q| new_quest_ids.contains(&q.config.id))
            .cloned()
            .collect();

        for notifier in notifiers {
            notifier
                .notify_full(&full_new_quests)
                .await
                .map_err(|e| format!("failed to send notifications: {e}"))?;
        }
    }

    let merged_quests = dedupe_by_key(&all_filtered_quests, |q| q.id.clone());
    storage::save_quests(&merged_quests).map_err(|e| format!("failed to save quests: {e}"))?;

    Ok(())
}
