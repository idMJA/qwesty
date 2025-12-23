use crate::models::{NotifyError, Quest};
use crate::utils::{parse_color, parse_timestamp, DEFAULT_REWARD_URL};
use log::{debug, error, info};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

const DISCORD_CDN: &str = "https://cdn.discordapp.com/";

#[derive(Clone)]
pub struct WebhookNotifier {
    name: Option<String>,
    webhook_url: String,
    client: reqwest::Client,
}

impl WebhookNotifier {
    #[must_use]
    pub fn new(webhook_url: String, name: Option<String>) -> Self {
        Self {
            name,
            webhook_url,
            client: reqwest::Client::new(),
        }
    }

    /// Send notification for full quest details
    ///
    /// # Errors
    /// Returns `NotifyError` if webhook request fails.
    pub async fn notify_full(&self, quests: &[Quest]) -> Result<(), NotifyError> {
        if quests.is_empty() {
            debug!("no quests to notify");
            return Ok(());
        }

        for quest in quests {
            self.send_full_quest_notification(quest).await?;
        }

        info!("sent {} full notifications", quests.len());
        Ok(())
    }

    async fn send_full_quest_notification(&self, quest: &Quest) -> Result<(), NotifyError> {
        let config = &quest.config;
        let color = parse_color(&config.colors.primary, 0x0058_65F2);
        let hero_url = format!("{}{}", DISCORD_CDN, config.assets.hero);
        let quest_url = format!("https://discord.com/quests/{}", config.id);

        let platforms = config
            .task_config_v2
            .as_ref()
            .map_or_else(|| String::from("Cross Platform"), build_platform_list);

        let tasks_desc = build_tasks_desc(config.task_config_v2.as_ref());
        let rewards_desc = build_rewards_desc(&config.rewards_config);
        let reward_media_url = reward_media_url(&config.rewards_config);
        let features_str = format_features(&config.features);
        let quest_info = build_quest_info(config, &platforms, &features_str);

        let content = WebhookContent {
            config,
            hero_url: &hero_url,
            quest_url: &quest_url,
            quest_info: &quest_info,
            tasks_desc: &tasks_desc,
            rewards_desc: &rewards_desc,
            reward_media_url: &reward_media_url,
            color,
        };

        let container = build_webhook_container(&content);

        let payload = json!({ "components": [container], "flags": 32768 });

        let notifier_name = self.name.as_deref().unwrap_or("default");
        debug!(
            "sending webhook (notifier={}) type={}, accent={}",
            notifier_name, container["type"], container["accent_color"]
        );
        debug!(
            "payload: {}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );

        let webhook_url_with_params = format!("{}?with_components=true", self.webhook_url);

        let response = self
            .client
            .post(&webhook_url_with_params)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(NotifyError::SendFailed)?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!(
                "webhook failed for notifier={}: status={status}, body={body}. payload: {}",
                notifier_name,
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
            return Ok(());
        }

        debug!("webhook response: status={status}");

        info!(
            "sent full notification for quest: {} to notifier: {}",
            config.messages.quest_name, notifier_name
        );

        // small pause to avoid hammering the webhook rate limits
        () = sleep(Duration::from_millis(250)).await;
        Ok(())
    }
}

fn build_platform_list(task_config: &crate::models::QuestTaskConfigV2) -> String {
    let mut platforms = Vec::new();

    for task in task_config.tasks.values() {
        let platform = match task.r#type.as_str() {
            "PLAY_ON_DESKTOP" => "🖥️ PC",
            "PLAY_ON_XBOX" => "🎮 Xbox",
            "PLAY_ON_PLAYSTATION" => "🎮 PlayStation",
            "WATCH_VIDEO" => "📺 Desktop",
            "WATCH_VIDEO_ON_MOBILE" => "📱 Mobile",
            _ => continue,
        };

        if !platforms.contains(&platform) {
            platforms.push(platform);
        }
    }

    platforms.join(", ")
}

fn reward_media_url(rewards_config: &crate::models::QuestRewardsConfig) -> String {
    rewards_config
        .rewards
        .first()
        .and_then(|r| r.asset.as_ref())
        .map_or_else(
            || DEFAULT_REWARD_URL.to_string(),
            |asset| {
                if asset.starts_with("quests/") {
                    // quests/<quest_id>/<file>.mp4  -> append ?format=png to force image
                    format!("https://cdn.discordapp.com/{asset}?format=png")
                } else {
                    asset.clone()
                }
            },
        )
}

fn build_tasks_desc(task_config: Option<&crate::models::QuestTaskConfigV2>) -> String {
    use std::fmt::Write;

    task_config.map_or_else(
        || String::from("## Tasks\n\nN/A"),
        |cfg| {
            let mut desc =
                String::from("## Tasks\n\nUsers must complete any of the following tasks");
            for (task_type, task) in &cfg.tasks {
                let platform = match task_type.as_str() {
                    "PLAY_ON_DESKTOP" => "Play on desktop",
                    "PLAY_ON_XBOX" => "Play on Xbox",
                    "PLAY_ON_PLAYSTATION" => "Play on PlayStation",
                    "WATCH_VIDEO" => "Watch video",
                    "WATCH_VIDEO_ON_MOBILE" => "Watch video on mobile",
                    _ => task_type.as_str(),
                };
                // target is in seconds; display minutes rounded up (59s -> 1 minute)
                let minutes = task.target.div_ceil(60);
                let unit = if minutes == 1 { "minute" } else { "minutes" };
                let _ = writeln!(desc, "\n- {platform} ({minutes} {unit})");
            }
            desc
        },
    )
}

fn build_rewards_desc(rewards_config: &crate::models::QuestRewardsConfig) -> String {
    use std::fmt::Write;

    rewards_config.rewards.first().map_or_else(
        || String::from("## Rewards\n\nN/A"),
        |reward| {
            let reward_type = match reward.r#type {
                1 => "In-game Code",
                2 => "Profile Decoration",
                3 => "Avatar Decoration",
                4 => "Virtual Currency",
                _ => "Unknown",
            };

            let mut desc = format!(
                "## Rewards\n\n**Type:** {reward_type}\n**SKU ID:** `{}`",
                reward.sku_id
            );

            let _ = writeln!(desc, "\n**Name:** {}", reward.messages.name);

            if let Some(orb_qty) = reward.orb_quantity {
                let _ = writeln!(desc, "\n**Orb Amount:** {orb_qty}");
            }

            desc
        },
    )
}

fn format_features(features: &[u32]) -> String {
    features
        .iter()
        .map(|f| {
            let feature_name = match f {
                3 => "QUEST_BAR_V2",
                9 => "REWARD_HIGHLIGHTING",
                13 => "DISMISSAL_SURVEY",
                14 => "MOBILE_QUEST_DOCK",
                15 => "QUESTS_CDN",
                16 => "PACING_CONTROLLER",
                18 => "VIDEO_QUEST_FORCE_HLS_VIDEO",
                19 => "VIDEO_QUEST_FORCE_END_CARD_CTA_SWAP",
                23 => "MOBILE_ONLY_QUEST_PUSH_TO_MOBILE",
                26 => "QUEST_VIDEO_HERO",
                _ => "UNKNOWN",
            };
            format!("`{feature_name}`")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_quest_info(
    config: &crate::models::QuestConfig,
    platforms: &str,
    features_str: &str,
) -> String {
    format!(
        "## Quest Info\n\n**Started at:** <t:{}:D>\n**Ended at:** <t:{}:D>\n**Platforms:** {}\n**Games:** {}\n**Applications:** [{}]({}) `{}`\n**Features:** {}\n",
        parse_timestamp(&config.starts_at),
            parse_timestamp(&config.expires_at),
        platforms,
        config.messages.game_title,
        config.application.name,
        config.application.link,
        config.application.id,
        features_str
    )
}

struct WebhookContent<'a> {
    config: &'a crate::models::QuestConfig,
    hero_url: &'a str,
    quest_url: &'a str,
    quest_info: &'a str,
    tasks_desc: &'a str,
    rewards_desc: &'a str,
    reward_media_url: &'a str,
    color: u32,
}

fn build_webhook_container(content: &WebhookContent) -> serde_json::Value {
    json!({
        "type": 17,
        "accent_color": content.color,
        "spoiler": false,
        "components": [
            {
                "type": 10,
                "content": format!(
                    "## [{}]({})",
                    content.config.messages.quest_name,
                    content.quest_url
                )
            },
            {
                "type": 12,
                "items": [{
                    "media": {
                        "url": content.hero_url
                    },
                    "description": null,
                    "spoiler": false
                }]
            },
            {
                "type": 14,
                "divider": true,
                "spacing": 1
            },
            {
                "type": 10,
                "content": content.quest_info
            },
            {
                "type": 14,
                "divider": true,
                "spacing": 1
            },
            {
                "type": 10,
                "content": content.tasks_desc
            },
            {
                "type": 14,
                "divider": true,
                "spacing": 1
            },
            {
                "type": 9,
                "accessory": {
                    "type": 11,
                    "media": {
                            "url": content.reward_media_url
                    },
                    "description": null,
                    "spoiler": false
                },
                "components": [{
                    "type": 10,
                        "content": content.rewards_desc
                }]
            },
            {
                "type": 14,
                "divider": true,
                "spacing": 1
            },
            {
                "type": 10,
                "content": format!("-# Quest ID: `{}`", content.config.id)
            },
            {
                "type": 14,
                "divider": true,
                "spacing": 1
            },
            {
                "type": 1,
                "components": [{
                    "type": 2,
                    "style": 5,
                    "label": content
                        .config
                        .cta_config
                        .as_ref()
                        .map_or("Go To Quests", |c| c.button_label.as_str()),
                    "emoji": {
                        "name": "🚀",
                        "id": null
                    },
                    "disabled": false,
                    "url": content.quest_url
                }]
            }
        ]
    })
}
