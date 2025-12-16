use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;

/// Ensure parent directory exists, creating it if needed.
///
/// # Errors
/// Returns `std::io::Error` if directory creation fails.
pub fn ensure_parent_dir(path: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = Path::new(path).parent() {
        if parent != Path::new("") {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

/// Read and deserialize JSON from file.
///
/// # Errors
/// Returns error if file cannot be read or JSON is invalid.
pub fn read_json_file<T: DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let s = fs::read_to_string(path)?;
    let t = serde_json::from_str(&s)?;
    Ok(t)
}

/// Write data as JSON to file.
///
/// # Errors
/// Returns error if file cannot be written or JSON serialization fails.
pub fn write_json_file<T: Serialize + ?Sized>(
    path: &str,
    data: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_parent_dir(path)?;
    let json = serde_json::to_string_pretty(data)?;
    fs::write(path, json)?;
    Ok(())
}
