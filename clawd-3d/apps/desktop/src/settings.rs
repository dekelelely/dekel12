//! What Clawd remembers between runs: settings.json in the app's config folder. The keys are the ones the Windows
//! app used when it was built with Electron, so its settings carry over the first time the Rust app starts.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const SIZES: [(&str, f64); 3] = [("Small", 243.0), ("Medium", 343.0), ("Large", 471.0)];

/// The app window's spot and size, in logical pixels.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub maximized: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Floating size: Small, Medium or Large.
    pub size: String,
    /// Floating Clawd's window, in logical pixels (null until he is first moved).
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub app: Option<AppBounds>,
    /// Start at login (floating, without the app window). Named after the Windows app's menu item.
    #[serde(rename = "startWithWindows")]
    pub start_at_login: Option<bool>,
    /// Anything else in the file (from another version) is kept as it was.
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            size: "Medium".into(),
            x: None,
            y: None,
            width: None,
            height: None,
            app: None,
            start_at_login: None,
            other: Default::default(),
        }
    }
}

impl Settings {
    /// Reads `dir/settings.json`, or on the first run the Electron app's settings in `legacy_dir`.
    pub fn load(dir: &Path, legacy_dir: Option<&Path>) -> Settings {
        let read =
            |d: &Path| -> Option<Settings> { serde_json::from_slice(&fs::read(d.join("settings.json")).ok()?).ok() };
        let mut s = read(dir).or_else(|| legacy_dir.and_then(read)).unwrap_or_default();
        if !SIZES.iter().any(|(name, _)| *name == s.size) {
            s.size = "Medium".into();
        }
        s
    }

    /// Writes the file whole (a temporary file renamed over it), so a crash never leaves half a file.
    pub fn save(&self, dir: &Path) {
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(dir)?;
            let tmp: PathBuf = dir.join("settings.json.tmp");
            fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
            fs::rename(&tmp, dir.join("settings.json"))
        };
        if let Err(err) = write() {
            eprintln!("Clawd: couldn't save settings: {err}");
        }
    }

    /// Floating Clawd's window width in logical pixels; the window is wide and short so his jumps and travel fit.
    pub fn pet_width(&self) -> f64 {
        SIZES.iter().find(|(name, _)| *name == self.size).map_or(343.0, |(_, w)| *w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_electron_apps_settings_and_keeps_unknown_keys() {
        let dir = std::env::temp_dir().join(format!("clawd-settings-{}", std::process::id()));
        let legacy = dir.join("legacy");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(
            legacy.join("settings.json"),
            r#"{"size":"Large","x":100,"y":200,"width":471,"height":306,"app":{"x":1,"y":2,"width":960,"height":620,"maximized":true},"startWithWindows":true,"future":42}"#,
        )
        .unwrap();
        let s = Settings::load(&dir, Some(&legacy));
        assert_eq!(s.size, "Large");
        assert_eq!((s.x, s.y), (Some(100.0), Some(200.0)));
        assert!(s.app.as_ref().unwrap().maximized);
        assert_eq!(s.start_at_login, Some(true));
        assert_eq!(s.pet_width(), 471.0);
        s.save(&dir);
        let again = Settings::load(&dir, None);
        assert_eq!(again.other.get("future"), Some(&serde_json::json!(42)));
        assert_eq!(again.start_at_login, Some(true));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_size_falls_back_to_medium() {
        let dir = std::env::temp_dir().join(format!("clawd-settings-size-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("settings.json"), r#"{"size":"Huge"}"#).unwrap();
        assert_eq!(Settings::load(&dir, None).size, "Medium");
        fs::remove_dir_all(&dir).unwrap();
    }
}
