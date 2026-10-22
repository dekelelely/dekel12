//! Updates from GitHub Releases, as in the Android app: no account, key or paid service. The "Clawd apps" workflow
//! publishes every build as release v1.N.0; the app compares N with its own version, downloads the build for this
//! system in the background and installs it on "Restart to update" (or when Clawd quits). Builds that can't replace
//! themselves (the portable .exe, the .deb, a copy run from the disk image) open the Releases page instead.
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const LATEST_API: &str = "https://api.github.com/repos/dekelelely/dekel12/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/dekelelely/dekel12/releases/latest";

/// What the update button and menu show. state: off, portable, idle, checking, none, downloading, ready or error.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub state: &'static str,
    pub current: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub version: String,
    pub percent: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
}

/// How this copy of Clawd was installed, which decides how it updates.
#[derive(Clone, Debug, PartialEq)]
pub enum Install {
    /// Windows, from Clawd-Setup: run the new installer silently.
    Installer,
    /// macOS: swap in the new Clawd.app next to this one.
    AppBundle(PathBuf),
    /// Linux: replace the AppImage file.
    AppImage(PathBuf),
    /// Can't replace itself: offer the Releases page.
    Portable,
    /// `cargo run` / `npm start`: no updates.
    Dev,
}

pub fn detect(appimage: Option<PathBuf>) -> Install {
    if cfg!(debug_assertions) {
        return Install::Dev;
    }
    let Ok(exe) = std::env::current_exe() else { return Install::Portable };
    if cfg!(windows) {
        // The installer puts uninstall.exe next to Clawd.exe; the portable .exe stands alone.
        return if exe.with_file_name("uninstall.exe").exists() { Install::Installer } else { Install::Portable };
    }
    if cfg!(target_os = "macos") {
        let bundle = exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf);
        return match bundle {
            Some(b) if !b.starts_with("/Volumes") && b.parent().is_some_and(writable) => Install::AppBundle(b),
            _ => Install::Portable,
        };
    }
    match appimage {
        Some(p) if p.parent().is_some_and(writable) => Install::AppImage(p),
        _ => Install::Portable,
    }
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".clawd-write-test-{}", std::process::id()));
    let ok = fs::write(&probe, b"").is_ok();
    let _ = fs::remove_file(probe);
    ok
}

/// The build number N of a version or tag "1.N.0" / "v1.N.0".
pub fn build_of(version: &str) -> Option<u32> {
    let mut parts = version.trim_start_matches('v').split('.');
    (parts.next()? == "1").then_some(())?;
    parts.next()?.parse().ok()
}

/// The release asset this system updates from.
pub fn asset_for(install: &Install, names: &[String], version: &str) -> Option<String> {
    let want = |name: &String| match install {
        Install::Installer => *name == format!("Clawd-Setup-{version}.exe"),
        Install::AppBundle(_) => name == "Clawd-macOS.app.tar.gz",
        Install::AppImage(_) => name.ends_with(&format!("{}.AppImage", std::env::consts::ARCH)),
        _ => false,
    };
    names.iter().find(|n| want(n)).cloned()
}

/// HTTPS with the system's own TLS and certificates (SChannel, Security.framework, OpenSSL).
fn agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build(),
        )
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_body(Some(Duration::from_secs(600)))
        .user_agent("Clawd-Desktop")
        .build()
        .into()
}

/// The newest release: its version ("1.N.0") and the download URL of each asset by name.
pub fn latest() -> Result<(String, Vec<(String, String)>), String> {
    latest_from(LATEST_API)
}

fn latest_from(api: &str) -> Result<(String, Vec<(String, String)>), String> {
    let release: serde_json::Value = agent()
        .get(api)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    let tag = release["tag_name"].as_str().unwrap_or_default();
    let build = build_of(tag).ok_or_else(|| format!("unexpected release tag {tag:?}"))?;
    let assets = release["assets"].as_array().map(Vec::as_slice).unwrap_or_default();
    let assets = assets
        .iter()
        .filter_map(|a| Some((a["name"].as_str()?.to_string(), a["browser_download_url"].as_str()?.to_string())))
        .collect();
    Ok((format!("1.{build}.0"), assets))
}

/// Downloads `url` into `to`, reporting whole percents as they change.
pub fn download(url: &str, to: &Path, mut progress: impl FnMut(u32)) -> Result<(), String> {
    let mut response =
        agent().get(url).header("Accept", "application/octet-stream").call().map_err(|e| e.to_string())?;
    let total = response.body().content_length().unwrap_or(0);
    let mut body = response.body_mut().with_config().limit(1 << 30).reader();
    let tmp = to.with_extension("part");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let (mut done, mut shown, mut buf) = (0u64, u32::MAX, vec![0u8; 64 * 1024]);
    let mut last = Instant::now();
    loop {
        let n = body.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        let percent = (done * 100).checked_div(total).unwrap_or(0) as u32;
        if percent != shown && (last.elapsed() > Duration::from_millis(150) || percent == 100) {
            shown = percent;
            last = Instant::now();
            progress(percent);
        }
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&tmp, to).map_err(|e| e.to_string())
}

/// Where downloads wait until they are installed: next to the app on macOS and Linux, so the swap is a rename on the
/// same disk; the temp folder on Windows, where the installer does the work.
pub fn staging_dir(install: &Install) -> PathBuf {
    match install {
        Install::AppBundle(b) => b.with_file_name(".Clawd-update"),
        Install::AppImage(p) => p.with_file_name(".Clawd-update"),
        _ => std::env::temp_dir().join("Clawd-update"),
    }
}

/// Unpacks a downloaded macOS update; returns the new Clawd.app.
pub fn unpack_app(archive: &Path, into: &Path) -> Result<PathBuf, String> {
    let _ = fs::remove_dir_all(into.join("Clawd.app"));
    let ok = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .status()
        .map_err(|e| e.to_string())?;
    let app = into.join("Clawd.app");
    if ok.success() && app.join("Contents").exists() {
        Ok(app)
    } else {
        Err("the update archive has no Clawd.app".into())
    }
}

/// Puts `new` where `current` is (an app bundle or an AppImage), keeping the old one until the new one is in place.
pub fn swap(current: &Path, new: &Path) -> Result<(), String> {
    let old = current.with_file_name(".Clawd-old");
    let _ = fs::remove_dir_all(&old);
    let _ = fs::remove_file(&old);
    fs::rename(current, &old).map_err(|e| format!("couldn't move the old version aside: {e}"))?;
    if let Err(e) = fs::rename(new, current) {
        let _ = fs::rename(&old, current);
        return Err(format!("couldn't put the new version in place: {e}"));
    }
    let _ = fs::remove_dir_all(&old);
    let _ = fs::remove_file(&old);
    Ok(())
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_and_assets() {
        assert_eq!(build_of("v1.23.0"), Some(23));
        assert_eq!(build_of("1.7.0"), Some(7));
        assert_eq!(build_of("v2.1.0"), None);
        let names: Vec<String> = [
            "Clawd.apk",
            "Clawd-Setup-1.23.0.exe",
            "Clawd-Portable-1.23.0.exe",
            "Clawd-macOS.app.tar.gz",
            "Clawd-1.23.0-x86_64.AppImage",
            "latest.yml",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(asset_for(&Install::Installer, &names, "1.23.0").as_deref(), Some("Clawd-Setup-1.23.0.exe"));
        assert_eq!(
            asset_for(&Install::AppBundle("/Applications/Clawd.app".into()), &names, "1.23.0").as_deref(),
            Some("Clawd-macOS.app.tar.gz")
        );
        if std::env::consts::ARCH == "x86_64" {
            assert_eq!(
                asset_for(&Install::AppImage("/x/Clawd.AppImage".into()), &names, "1.23.0").as_deref(),
                Some("Clawd-1.23.0-x86_64.AppImage")
            );
        }
        assert_eq!(asset_for(&Install::Portable, &names, "1.23.0"), None);
    }

    #[test]
    fn swap_keeps_the_old_one_until_the_new_one_is_in() {
        let dir = std::env::temp_dir().join(format!("clawd-swap-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (current, new) = (dir.join("Clawd.AppImage"), dir.join("new.AppImage"));
        fs::write(&current, b"old").unwrap();
        fs::write(&new, b"new").unwrap();
        swap(&current, &new).unwrap();
        assert_eq!(fs::read(&current).unwrap(), b"new");
        assert!(!new.exists() && !dir.join(".Clawd-old").exists());
        assert!(swap(&current, &dir.join("missing")).is_err());
        assert_eq!(fs::read(&current).unwrap(), b"new"); // put back after the failed swap
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A release and its asset served from this computer, the asset behind a redirect as on GitHub.
    #[test]
    fn reads_a_release_and_downloads_its_asset() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let asset = vec![7u8; 300_000];
        let (base2, asset2) = (base.clone(), asset.clone());
        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let response = match request.url() {
                    "/latest" => tiny_http::Response::from_string(format!(
                        r#"{{"tag_name":"v1.42.0","assets":[{{"name":"Clawd.apk","browser_download_url":"{base2}/apk"}},
                           {{"name":"Clawd-macOS.app.tar.gz","browser_download_url":"{base2}/redirect"}}]}}"#
                    )),
                    "/redirect" => tiny_http::Response::from_string("")
                        .with_status_code(302)
                        .with_header(tiny_http::Header::from_bytes("Location", format!("{base2}/file")).unwrap()),
                    "/file" => tiny_http::Response::from_data(asset2.clone()).with_chunked_threshold(usize::MAX),
                    _ => tiny_http::Response::from_string("").with_status_code(404),
                };
                let _ = request.respond(response);
            }
        });
        let (version, assets) = latest_from(&format!("{base}/latest")).unwrap();
        assert_eq!(version, "1.42.0");
        let names: Vec<String> = assets.iter().map(|(n, _)| n.clone()).collect();
        let name = asset_for(&Install::AppBundle("/Applications/Clawd.app".into()), &names, &version).unwrap();
        let url = &assets.iter().find(|(n, _)| *n == name).unwrap().1;
        let to = std::env::temp_dir().join(format!("clawd-download-{}", std::process::id()));
        let mut seen = Vec::new();
        download(url, &to, |p| seen.push(p)).unwrap();
        assert_eq!(fs::read(&to).unwrap(), asset);
        assert_eq!(seen.last(), Some(&100));
        fs::remove_file(&to).unwrap();
        assert!(latest_from(&format!("{base}/missing")).is_err());
    }

    /// Reads the real latest release (network): `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn reads_the_latest_release() {
        let (version, assets) = latest().unwrap();
        assert!(build_of(&version).is_some());
        assert!(assets.iter().any(|(n, _)| n == "Clawd.apk"), "{assets:?}");
    }
}
