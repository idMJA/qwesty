use crate::models::{Quest, StoredQuest};
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
        wait_before_next_locale(index, locale).await;
        info!("checking quests for locale: {locale}");

        let quests = fetch_locale_quests(client, token, locale).await?;
        let filtered_prefixed = prefix_filtered_quests(locale, &quests, reward_filter);
        let filtered_len = filtered_prefixed.len();
        let new_for_locale = calculate_new_for_locale(&filtered_prefixed, &stored, seed_only);

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
            let full_new_quests = select_full_quests_by_new_ids(&quests, &new_for_locale);
            notify_all(notifiers, &full_new_quests, locale).await?;
            persist_locale_new_quests(&mut stored, &new_for_locale, locale)?;
        }

        all_filtered_quests.extend(filtered_prefixed);
    }

    let saved = storage::load_stored_quests();
    let new_quests = storage::find_new_quests(&all_filtered_quests, &saved);
    info!("found {} new quests across all locales", new_quests.len());

    if seed_only {
        info!("initial run detected and initial_send_all=false; skipping notifications and seeding storage");
    } else if !new_quests.is_empty() {
        notify_cross_locale_new_quests(client, notifiers, token, &new_quests).await?;
    }

    let merged_quests = dedupe_by_key(&all_filtered_quests, |q| q.id.clone());
    storage::save_quests(&merged_quests).map_err(|e| format!("failed to save quests: {e}"))?;

    Ok(())
}

async fn wait_before_next_locale(index: usize, locale: &str) {
    if index > 0 {
        let delay_secs: u64 = 60 + u64::from(rand::random::<u8>() % 11); // 60..70
        info!("waiting {delay_secs} seconds before checking next locale ({locale})");
        sleep(Duration::from_secs(delay_secs)).await;
    }
}

async fn fetch_locale_quests(
    client: &QuestClient,
    token: &str,
    locale: &str,
) -> Result<Vec<Quest>, String> {
    client
        .fetch_quests_with_locale(token, locale)
        .await
        .map_err(|e| format!("failed to fetch quests for locale {locale}: {e}"))
}

fn prefix_filtered_quests(locale: &str, quests: &[Quest], reward_filter: &str) -> Vec<StoredQuest> {
    storage::filter_quests(quests, reward_filter)
        .into_iter()
        .map(|mut stored| {
            stored.id = format!("{}:{}", locale, stored.id);
            stored
        })
        .collect()
}

fn calculate_new_for_locale(
    filtered_prefixed: &[StoredQuest],
    stored: &[StoredQuest],
    seed_only: bool,
) -> Vec<StoredQuest> {
    if seed_only {
        return Vec::new();
    }

    let seen_base_ids = collect_base_ids(stored);
    let mut new_for_locale = storage::find_new_quests(filtered_prefixed, stored);
    new_for_locale.retain(|quest| !seen_base_ids.contains(base_quest_id(&quest.id)));
    new_for_locale
}

fn collect_base_ids(quests: &[StoredQuest]) -> HashSet<String> {
    quests
        .iter()
        .map(|quest| base_quest_id(&quest.id).to_string())
        .collect()
}

fn base_quest_id(id: &str) -> &str {
    id.split(':').next_back().unwrap_or(id)
}

fn select_full_quests_by_new_ids(quests: &[Quest], new_stored: &[StoredQuest]) -> Vec<Quest> {
    let new_ids = collect_base_ids(new_stored);

    quests
        .iter()
        .filter(|quest| new_ids.contains(quest.config.id.as_str()))
        .cloned()
        .collect()
}

async fn notify_all(
    notifiers: &[WebhookNotifier],
    full_new_quests: &[Quest],
    locale: &str,
) -> Result<(), String> {
    for notifier in notifiers {
        notifier
            .notify_full(full_new_quests)
            .await
            .map_err(|e| format!("failed to send notifications for locale {locale}: {e}"))?;
    }

    Ok(())
}

fn persist_locale_new_quests(
    stored: &mut Vec<StoredQuest>,
    new_for_locale: &[StoredQuest],
    locale: &str,
) -> Result<(), String> {
    stored.extend(new_for_locale.iter().cloned());
    let merged_local_stored = dedupe_by_key(stored, |quest| quest.id.clone());

    storage::save_quests(&merged_local_stored)
        .map_err(|e| format!("failed to save quests after locale {locale}: {e}"))
}

async fn notify_cross_locale_new_quests(
    client: &QuestClient,
    notifiers: &[WebhookNotifier],
    token: &str,
    new_quests: &[StoredQuest],
) -> Result<(), String> {
    let all_quests = client
        .fetch_quests_with_locale(token, "en-US")
        .await
        .map_err(|e| format!("failed to fetch quests for notifications: {e}"))?;

    let full_new_quests = select_full_quests_by_new_ids(&all_quests, new_quests);

    for notifier in notifiers {
        notifier
            .notify_full(&full_new_quests)
            .await
            .map_err(|e| format!("failed to send notifications: {e}"))?;
    }

    Ok(())
}
