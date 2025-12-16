use crate::models::{Quest, StoredQuest};
use crate::utils::{ensure_parent_dir, read_json_file, write_json_file};
use log::{debug, info, warn};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

static IN_MEMORY_QUESTS: LazyLock<Mutex<Vec<StoredQuest>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

static STORAGE_PATH: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("./known-quests.json".to_string()));

static STORAGE_TYPE: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new("json".to_string()));

/// Initialize storage backend.
///
/// # Panics
/// Panics if mutex locks are poisoned.
pub fn init_storage(storage_type: &str, storage_path: &str) {
    *STORAGE_TYPE.lock().unwrap() = storage_type.to_string();
    *STORAGE_PATH.lock().unwrap() = storage_path.to_string();

    if storage_type == "json" {
        let _ = ensure_parent_dir(storage_path);
    }

    info!("storage initialized - type: {storage_type}, path: {storage_path}");
}

#[must_use]
/// Load stored quests from backend.
///
/// # Panics
/// Panics if mutex locks are poisoned.
pub fn load_stored_quests() -> Vec<StoredQuest> {
    let storage_type = STORAGE_TYPE.lock().unwrap().clone();

    match storage_type.as_str() {
        "memory" => {
            let quests = IN_MEMORY_QUESTS.lock().unwrap().clone();
            debug!("loaded {} quests from in-memory storage", quests.len());
            quests
        }
        "json" => {
            let storage_path = STORAGE_PATH.lock().unwrap().clone();

            if !Path::new(&storage_path).exists() {
                return Vec::new();
            }

            match read_json_file::<Vec<StoredQuest>>(&storage_path) {
                Ok(quests) => {
                    debug!(
                        "loaded {} stored quests from {}",
                        quests.len(),
                        storage_path
                    );
                    quests
                }
                Err(e) => {
                    warn!("failed to read stored quests file: {e}");
                    Vec::new()
                }
            }
        }
        _ => {
            warn!("unknown storage type: {storage_type}");
            Vec::new()
        }
    }
}

/// Save quests to backend.
///
/// # Errors
/// Returns error if storage backend fails.
///
/// # Panics
/// Panics if mutex locks are poisoned.
pub fn save_quests(quests: &[StoredQuest]) -> Result<(), Box<dyn std::error::Error>> {
    let storage_type = STORAGE_TYPE.lock().unwrap().clone();

    match storage_type.as_str() {
        "memory" => {
            *IN_MEMORY_QUESTS.lock().unwrap() = quests.to_vec();
            info!("saved {} quests to in-memory storage", quests.len());
            Ok(())
        }
        "json" => {
            let storage_path = STORAGE_PATH.lock().unwrap().clone();

            ensure_parent_dir(&storage_path)?;
            write_json_file(&storage_path, quests)?;

            info!("saved {} quests to {}", quests.len(), storage_path);
            Ok(())
        }
        _ => Err(format!("unknown storage type: {storage_type}").into()),
    }
}

#[must_use]
pub fn filter_quests(quests: &[Quest], filter: &str) -> Vec<StoredQuest> {
    quests
        .iter()
        .filter_map(|quest| {
            let stored = StoredQuest::from(quest);

            match filter {
                "orbs" => {
                    if stored.reward_type == "orbs" {
                        Some(stored)
                    } else {
                        None
                    }
                }
                "decor" => {
                    if stored.reward_type == "decor" {
                        Some(stored)
                    } else {
                        None
                    }
                }
                "all" => Some(stored),
                _ => None,
            }
        })
        .collect()
}

#[must_use]
pub fn find_new_quests(
    all_quests: &[StoredQuest],
    stored_quests: &[StoredQuest],
) -> Vec<StoredQuest> {
    all_quests
        .iter()
        .filter(|q| !stored_quests.iter().any(|stored| stored.id == q.id))
        .cloned()
        .collect()
}
