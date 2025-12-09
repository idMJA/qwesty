use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;

pub fn ensure_parent_dir(path: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = Path::new(path).parent() {
        if parent != Path::new("") {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

pub fn read_json_file<T: DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let s = fs::read_to_string(path)?;
    let t = serde_json::from_str(&s)?;
    Ok(t)
}

pub fn write_json_file<T: Serialize + ?Sized>(
    path: &str,
    data: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_parent_dir(path)?;
    let json = serde_json::to_string_pretty(data)?;
    fs::write(path, json)?;
    Ok(())
}
