use crate::models::Rule;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

fn presets_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("presets");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create presets dir: {}", e))?;
    Ok(dir)
}

#[tauri::command]
pub fn save_preset(app: tauri::AppHandle, name: String, rules: Vec<Rule>) -> Result<(), String> {
    let dir = presets_dir(&app)?;
    let path = dir.join(format!("{}.json", name));
    let json = serde_json::to_string_pretty(&rules).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to save preset: {}", e))
}

#[tauri::command]
pub fn load_preset(app: tauri::AppHandle, name: String) -> Result<Vec<Rule>, String> {
    let dir = presets_dir(&app)?;
    let path = dir.join(format!("{}.json", name));
    let json = fs::read_to_string(path).map_err(|e| format!("Failed to load preset: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse preset: {}", e))
}

#[tauri::command]
pub fn list_presets(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = presets_dir(&app)?;
    let mut presets = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read presets dir: {}", e))?;
    for entry in entries.flatten() {
        if let Some(name) = entry.path().file_stem() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                presets.push(name.to_string_lossy().to_string());
            }
        }
    }
    presets.sort();
    Ok(presets)
}

#[tauri::command]
pub fn delete_preset(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let dir = presets_dir(&app)?;
    let path = dir.join(format!("{}.json", name));
    fs::remove_file(path).map_err(|e| format!("Failed to delete preset: {}", e))
}
