use crate::services::{storage, webhook::WebhookNotifier, QuestClient};
use crate::utils::dedupe_by_key;
use log::{debug, info};
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

        if new_for_locale.is_empty() {
            debug!("no new quests for locale {locale}");
        } else {
            info!(
                "found {} new quests for locale {locale}",
                new_for_locale.len()
            );

            // convert new stored quests to ids and find the full Quest objects for the locale
            let new_ids: Vec<String> = new_for_locale.iter().map(|q| q.id.clone()).collect();
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

            // append newly seen quests to stored and persist so subsequent locales wont re-notify that trash thing again
            stored.extend(new_for_locale.iter().cloned());
            let merged_local_stored = dedupe_by_key(&stored, |q| q.id.clone());
            storage::save_quests(&merged_local_stored)
                .map_err(|e| format!("failed to save quests after locale {locale}: {e}"))?;
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
            .map_err(|e| format!("failed to fetch quests for notifications: {e}"))?;

        let new_quest_ids: Vec<String> = new_quests.iter().map(|q| q.id.clone()).collect();
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
