//! Launch command construction and game process supervision.
use crate::error::{AppError, Result};
use crate::install;
use crate::java;
use crate::meta::{ArgValue, Argument, AssetIndex, VersionMeta};
use crate::paths::{path_str, GamePaths};
use crate::platform;
use crate::plan;
use crate::rules::{rules_allow, Env};
use crate::settings::{self, LastPlayed, Settings};
use crate::state::{AppState, GameHandle};
use crate::system;
use md5::{Digest, Md5};
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::oneshot;

const LAUNCHER_NAME: &str = "Diamond Launcher";
/// Shown as the version type in-game and sent to servers as the client brand.
const BRAND: &str = "Diamond Launcher";
const BRAND_AGENT_JAR: &[u8] = include_bytes!("../resources/diamond-agent.jar");

/// Writes the embedded brand agent next to the game files so the JVM can load it.
fn install_brand_agent(paths: &GamePaths) -> Result<PathBuf> {
    let path = paths.root.join("diamond-agent.jar");
    let current = std::fs::read(&path).ok();
    if current.as_deref() != Some(BRAND_AGENT_JAR) {
        std::fs::create_dir_all(&paths.root)?;
        std::fs::write(&path, BRAND_AGENT_JAR)?;
    }
    Ok(path)
}
const LOG_TAIL_LINES: usize = 40;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub version: String,
    /// starting | running | exited | crashed
    pub state: &'static str,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub log_tail: Vec<String>,
}

pub struct LaunchSpec {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// Same algorithm as Java's `UUID.nameUUIDFromBytes("OfflinePlayer:<name>")`.
pub fn offline_uuid(username: &str) -> String {
    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{username}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    let h = hex::encode(hash);
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// Splits user-provided arguments on whitespace, keeping double-quoted segments together.
pub fn split_args(input: &str) -> Vec<String> {
    let (mut args, mut current, mut in_quotes, mut started) = (Vec::new(), String::new(), false, false);
    for c in input.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                started = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

fn substitute(template: &str, vars: &HashMap<&str, String>) -> String {
    let re = Regex::new(r"\$\{([^}]+)\}").expect("valid regex");
    re.replace_all(template, |caps: &regex::Captures| {
        vars.get(&caps[1]).cloned().unwrap_or_else(|| caps[0].to_string())
    })
    .into_owned()
}

fn expand(args: &[Argument], env: &Env) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Argument::Plain(s) => out.push(s.clone()),
            Argument::Conditional { rules, value } if rules_allow(rules, env) => match value {
                ArgValue::One(s) => out.push(s.clone()),
                ArgValue::Many(list) => out.extend(list.iter().cloned()),
            },
            Argument::Conditional { .. } => {}
        }
    }
    out
}

pub struct LaunchInputs<'a> {
    pub meta: &'a VersionMeta,
    pub index: &'a AssetIndex,
    pub paths: &'a GamePaths,
    pub settings: &'a Settings,
    pub classpath: &'a [PathBuf],
    pub mods_dir: Option<&'a std::path::Path>,
    pub brand_agent: Option<&'a std::path::Path>,
    pub env: &'a Env,
    pub max_ram_mb: u32,
}

/// Builds the full JVM + game argument list purely from version metadata and settings.
pub fn build_args(i: &LaunchInputs) -> Vec<String> {
    let meta = i.meta;
    let sep = platform::classpath_separator();
    let mut classpath: Vec<String> = i.classpath.iter().map(|p| path_str(p)).collect();
    classpath.push(path_str(&i.paths.version_jar(&meta.id)));

    let index_id = meta.asset_index_id();
    let game_assets = if i.index.map_to_resources {
        i.paths.root.join("resources")
    } else if i.index.is_virtual {
        i.paths.virtual_assets(index_id)
    } else {
        i.paths.assets()
    };

    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", i.settings.username.clone()),
        ("version_name", meta.id.clone()),
        ("game_directory", path_str(&i.paths.root)),
        ("assets_root", path_str(&i.paths.assets())),
        ("assets_index_name", index_id.to_string()),
        ("game_assets", path_str(&game_assets)),
        ("auth_uuid", offline_uuid(&i.settings.username)),
        ("auth_access_token", "0".into()),
        ("auth_session", "0".into()),
        ("auth_xuid", "0".into()),
        ("clientid", "0".into()),
        ("user_type", "legacy".into()),
        ("user_properties", "{}".into()),
        ("version_type", BRAND.into()),
        ("natives_directory", path_str(&i.paths.natives(&meta.id))),
        ("launcher_name", LAUNCHER_NAME.into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", classpath.join(sep)),
        ("classpath_separator", sep.into()),
        ("library_directory", path_str(&i.paths.libraries())),
    ]);

    let (jvm_template, game_template): (Vec<String>, Vec<String>) = match &meta.arguments {
        Some(a) => (expand(&a.jvm, i.env), expand(&a.game, i.env)),
        None => (
            // Legacy metadata has no JVM arguments; these are the minimum every launcher must supply.
            vec!["-Djava.library.path=${natives_directory}".into(), "-cp".into(), "${classpath}".into()],
            split_args(meta.minecraft_arguments.as_deref().unwrap_or_default()),
        ),
    };

    let mut args = vec![format!("-Xmx{}M", i.max_ram_mb), "-Xms256M".to_string()];
    args.extend(split_args(&i.settings.jvm_args));
    if let Some(agent) = i.brand_agent {
        args.push(format!("-javaagent:{}={}", path_str(agent), BRAND));
    }
    if let Some(dir) = i.mods_dir {
        args.push(format!("-Dfabric.addMods={}", path_str(dir)));
    }
    if let Some(client) = meta.logging.as_ref().and_then(|l| l.client.as_ref()) {
        if let Some(id) = client.file.id.as_deref() {
            let path = path_str(&i.paths.log_config(id));
            args.push(client.argument.replace("${path}", &path));
        }
    }
    args.extend(jvm_template.iter().map(|a| substitute(a, &vars)));
    args.push(meta.main_class.clone());
    args.extend(game_template.iter().map(|a| substitute(a, &vars)));
    args
}

fn emit_status(app: &AppHandle, status: GameStatus) {
    let _ = app.emit("game-status", status);
}

fn status(version: &str, state: &'static str) -> GameStatus {
    GameStatus { version: version.to_string(), state, exit_code: None, message: None, log_tail: Vec::new() }
}

async fn drain<R: AsyncRead + Unpin>(reader: R, tail: Arc<Mutex<VecDeque<String>>>) {
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                if let Ok(mut t) = tail.lock() {
                    if t.len() == LOG_TAIL_LINES {
                        t.pop_front();
                    }
                    t.push_back(line);
                }
            }
        }
    }
}

pub async fn launch(app: &AppHandle, state: &AppState, id: &str) -> Result<()> {
    let settings = Settings::load();
    settings::validate_username(&settings.username)?;
    if state.installing.lock().map(|s| s.is_some()).unwrap_or(false) {
        return Err(AppError::new("busy", "An installation is in progress."));
    }
    if state.game.lock().map(|g| g.is_some()).unwrap_or(false) {
        return Err(AppError::new("busy", "The game is already running."));
    }

    let paths = settings.game_paths();
    let env = Env::current();
    let loaded = install::load_installed(&paths, id, &env, &settings).await?;
    let (meta, index) = (loaded.meta, loaded.index);
    let java = java::resolve(settings.java_path.as_deref(), &meta, &paths)?;
    let libs = plan::resolve_libraries(&meta, &env, &paths);

    let (total, available) = system::memory_mb();
    let max_ram_mb = settings
        .max_ram_mb
        .unwrap_or_else(|| system::recommended_ram_mb(total, available, platform::arch()))
        .min(total.min(u64::from(u32::MAX)) as u32);

    let agent = install_brand_agent(&paths)?;
    let args = build_args(&LaunchInputs {
        meta: &meta,
        index: &index,
        paths: &paths,
        settings: &settings,
        classpath: &libs.classpath,
        mods_dir: loaded.mods_dir.as_deref(),
        brand_agent: Some(&agent),
        env: &env,
        max_ram_mb,
    });
    let spec = LaunchSpec { java: PathBuf::from(&java.path), args, cwd: paths.root.clone() };

    std::fs::create_dir_all(&spec.cwd)?;
    let mut command = tokio::process::Command::new(&spec.java);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(platform::CREATE_NO_WINDOW);

    emit_status(app, status(id, "starting"));
    let mut child = command
        .spawn()
        .map_err(|e| AppError::new("launch_failed", format!("Could not start Java ({}): {e}", java.path)))?;

    let tail = Arc::new(Mutex::new(VecDeque::new()));
    let readers: Vec<_> = [
        child.stdout.take().map(|s| tokio::spawn(drain(s, tail.clone()))),
        child.stderr.take().map(|s| tokio::spawn(drain(s, tail.clone()))),
    ]
    .into_iter()
    .flatten()
    .collect();

    let (stop_tx, stop_rx) = oneshot::channel();
    if let Ok(mut slot) = state.game.lock() {
        *slot = Some(GameHandle { version: id.to_string(), stop: Some(stop_tx) });
    }

    let mut updated = settings.clone();
    updated.last_played = Some(LastPlayed {
        version: id.to_string(),
        at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    });
    let _ = updated.save();

    emit_status(app, status(id, "running"));

    let app = app.clone();
    let version = id.to_string();
    tokio::spawn(async move {
        let mut stopped_by_user = false;
        let exit = tokio::select! {
            r = child.wait() => r,
            _ = stop_rx => {
                stopped_by_user = true;
                let _ = child.kill().await;
                child.wait().await
            }
        };
        for r in readers {
            let _ = r.await;
        }
        let log_tail: Vec<String> = tail.lock().map(|t| t.iter().cloned().collect()).unwrap_or_default();

        let mut final_status = match exit {
            Ok(s) if s.success() || stopped_by_user => {
                let mut st = status(&version, "exited");
                st.exit_code = s.code();
                st
            }
            Ok(s) => {
                let mut st = status(&version, "crashed");
                st.exit_code = s.code();
                st.message = Some(format!("The game closed unexpectedly (exit code {}).", s.code().unwrap_or(-1)));
                st
            }
            Err(e) => {
                let mut st = status(&version, "crashed");
                st.message = Some(format!("Lost track of the game process: {e}"));
                st
            }
        };
        if final_status.state == "crashed" {
            final_status.log_tail = log_tail;
        }
        let state = app.state::<AppState>();
        if let Ok(mut slot) = state.game.lock() {
            *slot = None;
        }
        emit_status(&app, final_status);
    });
    Ok(())
}

pub fn stop(state: &AppState) -> Result<()> {
    let mut slot = state.game.lock().map_err(|_| AppError::new("internal", "State unavailable"))?;
    if let Some(handle) = slot.as_mut() {
        if let Some(tx) = handle.stop.take() {
            let _ = tx.send(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "downloads from Mojang and starts the real game; run with --ignored --nocapture"]
    async fn e2e_install_and_start_game() {
        let id = std::env::var("E2E_VERSION").unwrap_or_else(|_| "1.8.9".into());
        let state = AppState::new();
        let last = std::sync::Mutex::new(0u64);
        let emit: install::Emit = Arc::new(move |p| {
            let mut l = last.lock().unwrap();
            if p.phase != "download" || p.bytes_done / (50 << 20) > *l {
                *l = p.bytes_done / (50 << 20);
                println!("{} {}/{} bytes, {}/{} files", p.phase, p.bytes_done, p.bytes_total, p.files_done, p.files_total);
            }
        });
        install::install(emit, &state, &id, true).await.unwrap();

        let settings = Settings::load();
        let paths = settings.game_paths();
        let env = Env::current();
        let loaded = install::load_installed(&paths, &id, &env, &settings).await.unwrap();
        let (meta, index) = (loaded.meta, loaded.index);
        let java = java::resolve(settings.java_path.as_deref(), &meta, &paths).unwrap();
        println!("java: {} (major {})", java.path, java.major);
        let libs = plan::resolve_libraries(&meta, &env, &paths);
        let agent = install_brand_agent(&paths).unwrap();
        let args = build_args(&LaunchInputs {
            meta: &meta,
            index: &index,
            paths: &paths,
            settings: &settings,
            classpath: &libs.classpath,
            mods_dir: loaded.mods_dir.as_deref(),
            brand_agent: Some(&agent),
            env: &env,
            max_ram_mb: 2048,
        });
        let log = std::env::temp_dir().join(format!("diamond-e2e-{id}.log"));
        let out = std::fs::File::create(&log).unwrap();
        let mut child = std::process::Command::new(&java.path)
            .args(&args)
            .current_dir(&paths.root)
            .stdout(out.try_clone().unwrap())
            .stderr(out)
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(20));
        let alive = child.try_wait().unwrap().is_none();
        let _ = child.kill();
        println!("alive after 20s: {alive}\n{}", std::fs::read_to_string(&log).unwrap_or_default());
        assert!(alive, "game exited early, see {}", log.display());
    }

    #[test]
    fn offline_uuid_matches_java_algorithm() {
        // UUID.nameUUIDFromBytes("OfflinePlayer:Notch".getBytes()) is a well-known value.
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
    }

    #[test]
    fn splits_arguments_with_quotes() {
        assert_eq!(split_args("-Xss1M  -Dfoo=\"a b\" -XX:+UseG1GC"), vec!["-Xss1M", "-Dfoo=a b", "-XX:+UseG1GC"]);
        assert!(split_args("   ").is_empty());
    }

    fn fixture(json: &str) -> VersionMeta {
        serde_json::from_str(json).unwrap()
    }

    fn build(meta: &VersionMeta, index: &AssetIndex) -> Vec<String> {
        let env = Env { os: "windows", arch: "x86_64", os_version: "10.0".into() };
        let paths = GamePaths::new("/game");
        let settings = Settings { username: "Steve".into(), jvm_args: "-Dx=1".into(), ..Settings::default() };
        build_args(&LaunchInputs {
            meta,
            index,
            paths: &paths,
            settings: &settings,
            classpath: &[PathBuf::from("/game/libraries/a.jar")],
            mods_dir: None,
            brand_agent: None,
            env: &env,
            max_ram_mb: 2048,
        })
    }

    fn empty_index() -> AssetIndex {
        serde_json::from_str(r#"{"objects":{}}"#).unwrap()
    }

    #[test]
    fn modern_arguments_are_resolved_from_metadata() {
        let meta = fixture(
            r#"{"id":"1.20.4","type":"release","mainClass":"net.minecraft.client.main.Main",
            "assetIndex":{"id":"12","url":"u"},
            "arguments":{
              "jvm":["-Djava.library.path=${natives_directory}",{"rules":[{"action":"allow","os":{"name":"osx"}}],"value":"-XstartOnFirstThread"},"-cp","${classpath}"],
              "game":["--username","${auth_player_name}","--version","${version_name}",
                {"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"},
                "--assetsDir","${assets_root}","--assetIndex","${assets_index_name}"]}}"#,
        );
        let args = build(&meta, &empty_index());
        assert_eq!(args[0], "-Xmx2048M");
        assert!(args.contains(&"-Dx=1".to_string()));
        assert!(!args.iter().any(|a| a == "-XstartOnFirstThread" || a == "--demo"));
        let main = args.iter().position(|a| a == "net.minecraft.client.main.Main").unwrap();
        assert!(args[..main].iter().any(|a| a.starts_with("-Djava.library.path=") && a.contains("natives")));
        let cp = &args[args.iter().position(|a| a == "-cp").unwrap() + 1];
        assert!(cp.contains("a.jar") && cp.contains("1.20.4.jar"));
        assert!(args[main..].windows(2).any(|w| w == ["--username", "Steve"]));
        assert!(!args.iter().any(|a| a.contains("${")));
    }

    #[test]
    fn legacy_arguments_get_default_jvm_flags() {
        let meta = fixture(
            r#"{"id":"1.8.9","type":"release","mainClass":"net.minecraft.client.main.Main",
            "assetIndex":{"id":"1.8","url":"u"},
            "minecraftArguments":"--username ${auth_player_name} --gameDir ${game_directory} --assetsDir ${assets_root} --uuid ${auth_uuid} --accessToken ${auth_access_token}"}"#,
        );
        let args = build(&meta, &empty_index());
        assert!(args.iter().any(|a| a.starts_with("-Djava.library.path=")));
        assert!(args.contains(&"-cp".to_string()));
        assert!(args.windows(2).any(|w| w == ["--accessToken", "0"]));
        assert!(!args.iter().any(|a| a.contains("${")));
    }
}
