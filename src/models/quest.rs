use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub config: QuestConfig,
    pub user_status: Option<QuestUserStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestConfig {
    pub id: String,
    pub config_version: u32,
    pub starts_at: String,
    pub expires_at: String,
    pub features: Vec<u32>,
    pub application: QuestApplication,
    pub assets: QuestAssets,
    pub colors: QuestColors,
    pub messages: QuestMessages,
    pub task_config: Option<QuestTaskConfig>,
    pub task_config_v2: Option<QuestTaskConfigV2>,
    pub rewards_config: QuestRewardsConfig,
    pub cta_config: Option<QuestCtaConfig>,
    pub video_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestApplication {
    pub id: String,
    pub name: String,
    pub link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestAssets {
    pub hero: String,
    #[serde(default)]
    pub hero_video: Option<String>,
    pub quest_bar_hero: String,
    #[serde(default)]
    pub quest_bar_hero_video: Option<String>,
    #[serde(default)]
    pub game_tile: Option<String>,
    #[serde(default)]
    pub logotype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestColors {
    pub primary: String,
    pub secondary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestMessages {
    pub quest_name: String,
    pub game_title: String,
    pub game_publisher: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestTaskConfig {
    pub r#type: u32,
    pub join_operator: String,
    pub tasks: HashMap<String, QuestTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestTaskConfigV2 {
    pub tasks: HashMap<String, QuestTaskV2>,
    pub join_operator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestTask {
    pub event_name: String,
    pub target: u32,
    #[serde(default)]
    pub external_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestTaskV2 {
    pub r#type: String,
    pub target: u32,
    #[serde(default)]
    pub applications: Vec<serde_json::Value>,
    #[serde(default)]
    pub external_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestRewardsConfig {
    pub assignment_method: u32,
    pub rewards: Vec<QuestReward>,
    #[serde(default)]
    pub rewards_expire_at: Option<String>,
    pub platforms: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestReward {
    pub r#type: u32,
    pub sku_id: String,
    #[serde(default)]
    pub asset: Option<String>,
    pub messages: QuestRewardMessages,
    #[serde(default)]
    pub orb_quantity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestRewardMessages {
    pub name: String,
    pub name_with_article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestCtaConfig {
    pub link: String,
    pub button_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestUserStatus {
    pub user_id: String,
    pub quest_id: String,
    pub enrolled_at: String,
    pub completed_at: Option<String>,
    pub claimed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestsResponse {
    pub quests: Vec<Quest>,
    #[serde(default)]
    pub excluded_quests: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredQuest {
    pub id: String,
    pub name: String,
    pub reward: String,
    pub reward_type: String,
    pub expires_at: String,
    pub game_name: String,
}

impl From<&Quest> for StoredQuest {
    fn from(quest: &Quest) -> Self {
        let reward_type = determine_reward_type(&quest.config.rewards_config);
        let reward = get_reward_name(&quest.config.rewards_config);

        Self {
            id: quest.config.id.clone(),
            name: quest.config.messages.quest_name.clone(),
            reward,
            reward_type,
            expires_at: quest.config.expires_at.clone(),
            game_name: quest.config.messages.game_title.clone(),
        }
    }
}

fn determine_reward_type(rewards_config: &QuestRewardsConfig) -> String {
    if rewards_config.rewards.is_empty() {
        return "other".to_string();
    }

    let first_reward = &rewards_config.rewards[0];
    match first_reward.r#type {
        4 => "orbs".to_string(),
        3 => "decor".to_string(),
        1 => "code".to_string(),
        2 => "ingame".to_string(),
        _ => "other".to_string(),
    }
}

fn get_reward_name(rewards_config: &QuestRewardsConfig) -> String {
    rewards_config
        .rewards
        .first()
        .map_or_else(|| "Unknown Reward".to_string(), |r| r.messages.name.clone())
}
