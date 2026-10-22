//! Claude Code → Clawd. Claude Code runs hook commands at points in a session; Clawd's hook posts the hook's JSON to a
//! small server on this computer (127.0.0.1 only), and Clawd reacts: he waves when a session starts, types on his
//! laptop while Claude works, celebrates when it's done and jumps when it needs you. "React to Claude Code" in the menu adds the hook to Claude
//! Code's user settings (~/.claude/settings.json) or takes it out again.
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

pub const PORT: u16 = 47823;
/// The hook, the same for every event: it sends the event's JSON and never holds Claude up (2 s at most, and no
/// error when Clawd isn't running). curl comes with Windows 10 and later, macOS and Linux.
pub const HOOK_COMMAND: &str = "curl -s -m 2 --data-binary @- http://127.0.0.1:47823/claude || true";
/// The hook events Clawd listens to and what each means for him (the page's claudeEvent).
pub const EVENTS: [(&str, &str); 6] = [
    ("SessionStart", "hello"),
    ("UserPromptSubmit", "working"),
    ("PreToolUse", "working"),
    ("Stop", "done"),
    ("Notification", "attention"),
    ("SessionEnd", "idle"),
];

/// What a request means for Clawd: a hook's JSON (`hook_event_name`), or a path like /claude/done for scripts.
pub fn kind_of(path: &str, body: &[u8]) -> Option<&'static str> {
    let named = path.trim_end_matches('/').strip_prefix("/claude/").unwrap_or("");
    if !named.is_empty() {
        return ["hello", "working", "done", "attention", "idle"].into_iter().find(|k| *k == named);
    }
    let event: Value = serde_json::from_slice(body).ok()?;
    let name = event.get("hook_event_name")?.as_str()?;
    EVENTS.iter().find(|(e, _)| *e == name).map(|(_, kind)| *kind)
}

/// Serves hook calls on 127.0.0.1:PORT for as long as Clawd runs, passing each one's meaning to `react`.
pub fn serve(react: impl Fn(&'static str) + Send + 'static) {
    std::thread::spawn(move || {
        let server = match tiny_http::Server::http(("127.0.0.1", PORT)) {
            Ok(server) => server,
            Err(err) => return eprintln!("Clawd: Claude Code hooks are off, port {PORT} is taken: {err}"),
        };
        for mut request in server.incoming_requests() {
            // Web pages can reach 127.0.0.1 too; the hook (curl) sends no Origin, a browser always does.
            let from_browser = request.headers().iter().any(|h| h.field.equiv("Origin"));
            let mut body = Vec::new();
            let _ = request.as_reader().take(1 << 20).read_to_end(&mut body);
            let kind = if from_browser || !request.url().starts_with("/claude") {
                None
            } else {
                kind_of(request.url(), &body)
            };
            if let Some(kind) = kind {
                react(kind);
            }
            let status = if from_browser { 403 } else { 204 };
            let _ = request.respond(tiny_http::Response::empty(status));
        }
    });
}

/// Claude Code's user settings file: $CLAUDE_CONFIG_DIR/settings.json, or ~/.claude/settings.json.
pub fn settings_path(home: Option<PathBuf>) -> Option<PathBuf> {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir).join("settings.json")),
        _ => home.map(|h| h.join(".claude").join("settings.json")),
    }
}

fn is_ours(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| hooks.iter().any(|h| h.get("command").and_then(Value::as_str) == Some(HOOK_COMMAND)))
}

/// Whether Clawd's hook is in the settings, for every event.
pub fn installed(settings: &Value) -> bool {
    let hooks = settings.get("hooks");
    EVENTS.iter().all(|(event, _)| {
        hooks.and_then(|h| h.get(*event)).and_then(Value::as_array).is_some_and(|list| list.iter().any(is_ours))
    })
}

/// Adds Clawd's hook for every event it listens to, leaving everything else as it was.
pub fn add_hooks(settings: &mut Value) -> Result<(), String> {
    let root = settings.as_object_mut().ok_or("the settings file isn't a JSON object")?;
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in the settings isn't an object")?;
    for (event, _) in EVENTS {
        let list = hooks.entry(event).or_insert_with(|| Value::Array(Vec::new()));
        let list = list.as_array_mut().ok_or_else(|| format!("\"hooks.{event}\" in the settings isn't a list"))?;
        if !list.iter().any(is_ours) {
            let mut entry = json!({ "hooks": [{ "type": "command", "command": HOOK_COMMAND }] });
            match event {
                "PreToolUse" => entry["matcher"] = json!("*"),         // every tool
                "SessionStart" => entry["matcher"] = json!("startup"), // a new session, not a resumed or compacted one
                _ => {}
            }
            list.push(entry);
        }
    }
    Ok(())
}

/// Takes Clawd's hook out again, and any list or "hooks" object it leaves empty.
pub fn remove_hooks(settings: &mut Value) {
    let Some(root) = settings.as_object_mut() else { return };
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else { return };
    for (event, _) in EVENTS {
        if let Some(list) = hooks.get_mut(event).and_then(Value::as_array_mut) {
            list.retain(|entry| !is_ours(entry));
            if list.is_empty() {
                hooks.remove(event);
            }
        }
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
}

/// Reads the settings file: an empty object if there is none yet, an error if it can't be understood (it is then
/// left alone).
pub fn read_settings(path: &PathBuf) -> Result<Value, String> {
    match fs::read(path) {
        Ok(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => Ok(json!({})),
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("{} isn't valid JSON ({e})", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("couldn't read {}: {e}", path.display())),
    }
}

/// Writes the settings back, after keeping a copy of the file as it was (settings.json.clawd-backup).
pub fn write_settings(path: &PathBuf, settings: &Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if path.exists() {
        fs::copy(path, path.with_extension("json.clawd-backup"))
            .map_err(|e| format!("couldn't back up {}: {e}", path.display()))?;
    }
    let mut text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    text.push('\n');
    fs::write(path, text).map_err(|e| format!("couldn't write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_json_and_paths_mean_moves() {
        assert_eq!(kind_of("/claude", br#"{"hook_event_name":"UserPromptSubmit","prompt":"hi"}"#), Some("working"));
        assert_eq!(kind_of("/claude", br#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#), Some("working"));
        assert_eq!(kind_of("/claude", br#"{"hook_event_name":"Stop"}"#), Some("done"));
        assert_eq!(
            kind_of("/claude", br#"{"hook_event_name":"Notification","message":"Claude needs your permission"}"#),
            Some("attention")
        );
        assert_eq!(kind_of("/claude", br#"{"hook_event_name":"SessionEnd"}"#), Some("idle"));
        assert_eq!(kind_of("/claude", br#"{"hook_event_name":"SessionStart","source":"startup"}"#), Some("hello"));
        assert_eq!(kind_of("/claude", br#"{"hook_event_name":"SubagentStop"}"#), None);
        assert_eq!(kind_of("/claude", b"not json"), None);
        assert_eq!(kind_of("/claude/done", b""), Some("done"));
        assert_eq!(kind_of("/claude/dance", b""), None);
    }

    #[test]
    fn adds_and_removes_only_its_own_hooks() {
        let mut s = json!({
            "model": "opus",
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] }
        });
        assert!(!installed(&s));
        add_hooks(&mut s).unwrap();
        assert!(installed(&s));
        add_hooks(&mut s).unwrap(); // twice is the same as once
        assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(s["hooks"]["PreToolUse"][0]["matcher"], "*");
        remove_hooks(&mut s);
        assert!(!installed(&s));
        assert_eq!(
            s,
            json!({ "model": "opus", "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] } })
        );
        let mut empty = json!({});
        add_hooks(&mut empty).unwrap();
        remove_hooks(&mut empty);
        assert_eq!(empty, json!({}));
    }

    #[test]
    fn refuses_settings_it_cannot_understand() {
        assert!(add_hooks(&mut json!([])).is_err());
        assert!(add_hooks(&mut json!({ "hooks": [] })).is_err());
    }
}
