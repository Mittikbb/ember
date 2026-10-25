//! Ember: your Deadlock status in Discord, worded your way.
//!
//! Three threads share one state: `watch` follows the game and its log,
//! `data` fetches heroes, rank and match numbers from deadlock-api, and
//! `publish` keeps the Discord activity in line with both.

mod api;
mod discord;
mod log;
mod memory;
mod presence;
mod settings;
mod steam;

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{
    AppHandle, Manager, State, WindowEvent,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_opener::OpenerExt;

use api::{Badge, Catalog, MatchStats};
use log::{Phase, Tracker};
use presence::{Activity, Facts, HeroFacts, RankFacts};
use settings::{Lang, Settings, StateCfg};

/// Discord allows 5 updates per 20 seconds.
const MIN_SEND_GAP: Duration = Duration::from_secs(4);
/// How long the API is asked for a finished match's numbers.
const STATS_GIVE_UP: i64 = 30 * 60;
/// Match metadata may go to Steam, which allows 3 requests an hour.
const METADATA_GAP: Duration = Duration::from_secs(20 * 60);
/// Read at most this much of the log per tick, so a huge log can't stall us.
const READ_CHUNK: u64 = 8 << 20;
/// Key of the Rich Presence asset the user uploads to their Discord app.
const DEADLOCK_ASSET: &str = "deadlock";
/// Deadlock's 256 px icon, from the .ico Steam keeps for the game's shortcut.
const DEADLOCK_ICON: &[u8] = include_bytes!("../../public/deadlock-icon.png");

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordStatus {
    /// "off", "idle" (waiting for the game), "connecting", "ready" or "error".
    state: String,
    message: Option<String>,
    user: Option<String>,
}

#[derive(Default)]
struct Live {
    running: bool,
    citadel: Option<PathBuf>,
    account: Option<u32>,
    /// The game runs but hasn't written the log since it started: no -condebug.
    log_stale: bool,
    tracker: Option<Tracker>,
    catalog: Catalog,
    badge: Option<Badge>,
    stats: HashMap<u64, MatchStats>,
    discord: DiscordStatus,
    activity: Option<Activity>,
    lumen_clash: bool,
    game_icon: Option<String>,
    memory: Option<memory::LiveMatch>,
    memory_status: memory::MemoryStatus,
}

struct Shared {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    catalog_path: PathBuf,
    live: Mutex<Live>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    running: bool,
    deadlock_found: bool,
    log_stale: bool,
    facts: Facts,
    discord: DiscordStatus,
    /// What Discord shows right now.
    activity: Option<Activity>,
    /// The client id is the one Lumen uses.
    lumen_clash: bool,
    heroes_loaded: bool,
    memory: memory::MemoryStatus,
}

fn now() -> i64 {
    chrono::Local::now().timestamp()
}

/// What's known now, the way templates see it.
fn facts(live: &Live, s: &Settings, now: i64) -> Facts {
    let mut f = Facts {
        account_id: live.account,
        steam_id: live.account.map(steam::steam_id64),
        game_icon: if s.deadlock_asset { Some(DEADLOCK_ASSET.into()) } else { live.game_icon.clone() },
        ..Default::default()
    };
    let Some(t) = live.tracker.as_ref().filter(|_| live.running) else {
        return f;
    };
    f.phase = t.phase;
    f.since = t.since;
    f.match_start = t.match_start;
    f.queue_start = t.queue_start;
    f.match_id = t.match_id;

    let linger = s.post_game_minutes as i64 * 60;
    let finished = t.finished.as_ref();
    let post_game = t.phase == Phase::PostGame
        || (t.phase == Phase::Hideout && finished.is_some_and(|d| now - d.ended < linger));
    if post_game {
        let done = finished.unwrap();
        f.phase = Phase::PostGame;
        f.since = done.ended;
        f.match_id = done.match_id;
    }
    // The last match's numbers belong to the post-game and the hideout, not to the next match.
    let shows_last = post_game || t.phase == Phase::Hideout;
    f.stats = finished
        .filter(|_| shows_last)
        .and_then(|d| d.match_id)
        .and_then(|id| live.stats.get(&id))
        .cloned();
    f.played_s = finished.and_then(|d| Some((d.ended - d.started?).max(0) as u32));

    let from_stats = f.stats.as_ref().filter(|_| post_game).and_then(|s| live.catalog.hero_by_id(s.hero_id));
    let folder = if post_game { finished.and_then(|d| d.hero.as_deref()) } else { t.hero.as_deref() };
    f.hero = match (from_stats, folder) {
        (Some(h), _) => Some(hero_facts(h)),
        (None, Some(folder)) => Some(match live.catalog.hero_by_folder(folder) {
            Some(h) => hero_facts(h),
            None => HeroFacts { name: title_case(folder), ..Default::default() },
        }),
        (None, None) => None,
    };
    f.rank = live.badge.and_then(|b| rank_facts(&live.catalog, b));

    // Memory, when allowed: the exact hero, live numbers, and spectating.
    let fresh = live.memory.as_ref().filter(|m| s.memory && now - m.read_at <= memory::FRESH);
    if let Some(m) = fresh {
        if matches!(f.phase, Phase::Loading | Phase::Match | Phase::Sandbox) {
            if m.spectating && m.in_match {
                f.phase = Phase::Spectating;
            }
            if let Some(h) = m.hero_id.and_then(|id| live.catalog.hero_by_id(id)) {
                f.hero = Some(hero_facts(h));
            }
            f.live = Some(m.clone());
        }
    }
    f
}

fn hero_facts(h: &api::Hero) -> HeroFacts {
    HeroFacts {
        name: h.name.clone(),
        line: h.hideout_line.clone(),
        icon: h.icon.clone(),
        card: h.card.clone(),
    }
}

fn rank_facts(catalog: &Catalog, b: Badge) -> Option<RankFacts> {
    let r = catalog.rank(b.tier)?;
    let name = if b.subrank > 0 { format!("{} {}", r.name, b.subrank) } else { r.name.clone() };
    Some(RankFacts { name, image: r.image.clone() })
}

fn title_case(folder: &str) -> String {
    let mut c = folder.split('_').next().unwrap_or(folder).chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Fills what isn't known with believable examples, for the preview.
fn with_samples(mut f: Facts, live: &Live, phase: Phase, memory: bool, now: i64) -> Facts {
    let live_phase = f.phase == phase;
    let in_game = matches!(phase, Phase::Loading | Phase::Match | Phase::Sandbox | Phase::Spectating);
    if memory && in_game && f.live.is_none() {
        f.live = Some(memory::LiveMatch {
            read_at: now,
            in_match: true,
            spectating: phase == Phase::Spectating,
            hero_id: None,
            kills: Some(7),
            deaths: Some(2),
            assists: Some(11),
            souls: Some(24_130),
            level: Some(18),
            last_hits: Some(140),
            match_mode: Some(4),
            game_mode: Some(1),
            brawl: None,
        });
    }
    if f.hero.is_none() {
        f.hero = Some(
            live.catalog
                .hero_by_folder("werewolf")
                .or(live.catalog.heroes.first())
                .map(hero_facts)
                .unwrap_or_else(|| HeroFacts { name: "Silver".into(), ..Default::default() }),
        );
    }
    if f.rank.is_none() {
        f.rank = rank_facts(&live.catalog, Badge { tier: 7, subrank: 3 })
            .or_else(|| Some(RankFacts { name: "Ritualist 3".into(), image: None }));
    }
    if f.stats.is_none() {
        let hero_id = live.catalog.hero_by_folder("werewolf").map_or(80, |h| h.id);
        f.stats = Some(MatchStats {
            match_id: 123_807_324,
            hero_id,
            kills: 14,
            deaths: 4,
            assists: 9,
            souls: 31_240,
            last_hits: 212,
            level: 27,
            duration_s: 1928,
            won: true,
            game_mode: 1,
            match_mode: 4,
        });
    }
    if !live_phase {
        f.phase = phase;
        f.since = now - 58;
        f.match_start = Some(now - 754);
        f.queue_start = Some(now - 42);
    }
    f.match_id = f.match_id.or(Some(123_807_324));
    f.account_id = f.account_id.or(Some(1_484_768_650));
    f.steam_id = f.steam_id.or(Some(steam::steam_id64(1_484_768_650)));
    f
}

// ---- watch: the game process and its log --------------------------------

struct Watch {
    pid: Option<u32>,
    started: i64,
    offset: u64,
    carry: Vec<u8>,
    dir_key: Option<String>,
    dir_checked: Option<Instant>,
}

fn watch(app: AppHandle) {
    let shared = app.state::<Shared>();
    let mut w = Watch { pid: None, started: 0, offset: 0, carry: Vec::new(), dir_key: None, dir_checked: None };
    loop {
        watch_tick(&shared, &mut w);
        thread::sleep(Duration::from_secs(1));
    }
}

fn watch_tick(shared: &Shared, w: &mut Watch) {
    let custom = shared.settings.lock().unwrap().deadlock_dir.clone();
    let stale_dir = w.dir_checked.is_none_or(|t| t.elapsed() > Duration::from_secs(30));
    if w.dir_key.as_deref() != Some(custom.as_str()) || stale_dir {
        let dir = steam::citadel_dir(&custom);
        let account = dir.as_deref().and_then(steam::account_id);
        let icon = steam::deadlock_icon_url();
        let mut live = shared.live.lock().unwrap();
        live.citadel = dir;
        live.game_icon = Some(icon);
        if account.is_some() {
            live.account = account;
        }
        w.dir_key = Some(custom);
        w.dir_checked = Some(Instant::now());
    }

    let process = steam::game_process();
    let mut live = shared.live.lock().unwrap();
    let Some((pid, started)) = process else {
        live.running = false;
        live.log_stale = false;
        live.tracker = None;
        w.pid = None;
        return;
    };
    if w.pid != Some(pid) {
        w.pid = Some(pid);
        w.started = started;
        w.offset = 0;
        w.carry.clear();
        live.tracker = Some(Tracker::new(started));
    }
    live.running = true;
    let Some(log) = live.citadel.as_ref().map(|d| d.join("console.log")) else {
        return;
    };
    live.log_stale = steam::modified_unix(&log).is_none_or(|m| m < started - 5) && now() - started > 60;

    let Ok(mut file) = File::open(&log) else { return };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if len < w.offset {
        // Rewritten from the start.
        w.offset = 0;
        w.carry.clear();
        live.tracker = Some(Tracker::new(w.started));
    }
    if len == w.offset || file.seek(SeekFrom::Start(w.offset)).is_err() {
        return;
    }
    let mut buf = Vec::new();
    let Ok(read) = file.take(READ_CHUNK).read_to_end(&mut buf) else { return };
    w.offset += read as u64;
    w.carry.extend_from_slice(&buf);
    let Some(end) = w.carry.iter().rposition(|&b| b == b'\n') else { return };
    let complete: Vec<u8> = w.carry.drain(..=end).collect();
    let tracker = live.tracker.get_or_insert_with(|| Tracker::new(w.started));
    for line in complete.split(|&b| b == b'\n') {
        tracker.feed(&String::from_utf8_lossy(line));
    }
}

// ---- data: deadlock-api ---------------------------------------------------

fn data(app: AppHandle) {
    let shared = app.state::<Shared>();
    let mut catalog_tried: Option<Instant> = None;
    let mut badge_for: Option<(u32, Option<u64>)> = None;
    let mut badge_retry = Instant::now();
    let mut stats_retry: HashMap<u64, i64> = HashMap::new();
    let mut last_metadata: Option<Instant> = None;
    loop {
        // Heroes and ranks: on start and once a day, retried every 10 minutes.
        let fetched = shared.live.lock().unwrap().catalog.fetched;
        let due = fetched == 0 || now() - fetched > 24 * 3600;
        if due && catalog_tried.is_none_or(|t| t.elapsed() > Duration::from_secs(600)) {
            catalog_tried = Some(Instant::now());
            let catalog = Catalog::load(&shared.catalog_path, now());
            if !catalog.heroes.is_empty() {
                shared.live.lock().unwrap().catalog = catalog;
            }
        }

        let (account, finished) = {
            let live = shared.live.lock().unwrap();
            (live.account, live.tracker.as_ref().and_then(|t| t.finished.clone()))
        };

        // Rank: per account, again after every match.
        if let Some(account) = account {
            let key = (account, finished.as_ref().and_then(|f| f.match_id));
            if badge_for != Some(key) && Instant::now() >= badge_retry {
                match api::player_badge(account) {
                    Ok(badge) => {
                        shared.live.lock().unwrap().badge = badge;
                        badge_for = Some(key);
                    }
                    Err(e) => {
                        eprintln!("rank: {e}");
                        badge_retry = Instant::now() + Duration::from_secs(300);
                    }
                }
            }
        }

        // The finished match's numbers, once the API has them.
        if let (Some(account), Some(done)) = (account, finished) {
            if let Some(id) = done.match_id {
                let have = shared.live.lock().unwrap().stats.contains_key(&id);
                let next = stats_retry.entry(id).or_insert(done.ended + 20);
                if !have && now() >= *next && now() - done.ended < STATS_GIVE_UP {
                    *next = now() + 60;
                    let mut found = api::match_stats(account, id);
                    // History lags for most accounts; then ask for the match itself, rarely.
                    let waited = now() - done.ended >= 90;
                    if matches!(found, Ok(None)) && waited && last_metadata.is_none_or(|t| t.elapsed() >= METADATA_GAP) {
                        last_metadata = Some(Instant::now());
                        found = api::match_stats_from_metadata(account, id);
                    }
                    match found {
                        Ok(Some(stats)) => {
                            shared.live.lock().unwrap().stats.insert(id, stats);
                        }
                        Ok(None) => {}
                        Err(e) => eprintln!("match {id}: {e}"),
                    }
                }
            }
        }

        let clash = lumen_client_id().is_some_and(|id| id == shared.settings.lock().unwrap().client_id.trim());
        shared.live.lock().unwrap().lumen_clash = clash;

        thread::sleep(Duration::from_secs(3));
    }
}

/// Lumen's Discord client id, if Lumen is set up on this PC.
fn lumen_client_id() -> Option<String> {
    let path = PathBuf::from(std::env::var_os("APPDATA")?).join("dev.lumen.app").join("settings.json");
    let v: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let id = v["discord"]["clientId"].as_str()?.trim();
    (!id.is_empty()).then(|| id.to_string())
}

// ---- publish: the Discord activity ------------------------------------------

fn publish(app: AppHandle) {
    let shared = app.state::<Shared>();
    let mut conn: Option<discord::Ipc> = None;
    let mut conn_id = String::new();
    let mut sent: Option<Option<Activity>> = None;
    let mut last_send: Option<Instant> = None;
    let mut retry_at = Instant::now();

    let set_status = |state: &str, message: Option<String>, user: Option<String>| {
        shared.live.lock().unwrap().discord = DiscordStatus { state: state.into(), message, user };
    };

    loop {
        thread::sleep(Duration::from_millis(500));
        let s = shared.settings.lock().unwrap().clone();
        let (f, running) = {
            let live = shared.live.lock().unwrap();
            (facts(&live, &s, now()), live.running)
        };
        let client_id = s.client_id.trim().to_string();
        let want = s.enabled && !client_id.is_empty() && running;
        let desired = if want {
            s.states.get(&f.phase).and_then(|cfg| presence::build(cfg, &s.buttons, &f, s.lang))
        } else {
            None
        };

        if conn.is_some() && (!want || conn_id != client_id) {
            if let Some(c) = conn.as_mut() {
                let _ = c.set_activity(None);
            }
            conn = None;
            sent = None;
            shared.live.lock().unwrap().activity = None;
        }
        if !want {
            let (state, message) = if !s.enabled {
                ("off", None)
            } else if client_id.is_empty() {
                ("off", Some("discord.noClientId".to_string()))
            } else {
                ("idle", Some("discord.waitingGame".to_string()))
            };
            set_status(state, message, None);
            continue;
        }

        if conn.is_none() {
            if Instant::now() < retry_at {
                continue;
            }
            set_status("connecting", None, None);
            match discord::Ipc::connect(&client_id) {
                Ok(c) => {
                    set_status("ready", None, c.user.clone());
                    conn = Some(c);
                    conn_id = client_id.clone();
                }
                Err(e) => {
                    let state = if e == discord::NOT_RUNNING { "idle" } else { "error" };
                    set_status(state, Some(e), None);
                    retry_at = Instant::now() + Duration::from_secs(5);
                    continue;
                }
            }
        }
        let c = conn.as_mut().unwrap();
        let result = c.idle().and_then(|_| {
            let changed = sent.as_ref() != Some(&desired);
            if changed && last_send.is_none_or(|t| t.elapsed() >= MIN_SEND_GAP) {
                c.set_activity(desired.as_ref())?;
                last_send = Some(Instant::now());
                sent = Some(desired.clone());
                shared.live.lock().unwrap().activity = desired.clone();
            }
            Ok(())
        });
        if let Err(e) = result {
            conn = None;
            sent = None;
            shared.live.lock().unwrap().activity = None;
            set_status("error", Some(e), None);
            retry_at = Instant::now() + Duration::from_secs(5);
        }
    }
}

// ---- commands -------------------------------------------------------------

#[tauri::command]
fn get_settings(shared: State<Shared>) -> Settings {
    shared.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(shared: State<Shared>, settings: Settings, app: AppHandle) -> Result<(), String> {
    settings.save(&shared.settings_path)?;
    sync_tray(&app, &settings);
    *shared.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
fn get_status(shared: State<Shared>) -> Status {
    let s = shared.settings.lock().unwrap().clone();
    let live = shared.live.lock().unwrap();
    Status {
        running: live.running,
        deadlock_found: live.citadel.is_some(),
        log_stale: live.log_stale,
        facts: facts(&live, &s, now()),
        discord: live.discord.clone(),
        activity: live.activity.clone(),
        lumen_clash: live.lumen_clash,
        heroes_loaded: !live.catalog.heroes.is_empty(),
        memory: live.memory_status.clone(),
    }
}

/// The activity a phase would show with these settings: live values where
/// the game is in that phase, examples elsewhere.
#[tauri::command]
fn preview(shared: State<Shared>, settings: Settings, phase: Phase) -> Option<Activity> {
    let live = shared.live.lock().unwrap();
    let t = now();
    let f = with_samples(facts(&live, &settings, t), &live, phase, settings.memory, t);
    let cfg = settings.states.get(&phase).cloned().unwrap_or_default();
    presence::build(&StateCfg { enabled: true, ..cfg }, &settings.buttons, &f, settings.lang)
}

#[tauri::command]
fn preset(lang: Lang) -> BTreeMap<Phase, StateCfg> {
    settings::preset(lang)
}

#[tauri::command]
fn variables() -> Vec<&'static str> {
    presence::VARIABLES.to_vec()
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("only https links".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Saves the sharp Deadlock icon to Downloads and shows it in Explorer, for
/// uploading to the Discord app.
#[tauri::command]
fn save_icon(app: AppHandle) -> Result<(), String> {
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let path = dir.join("deadlock.png");
    fs::write(&path, DEADLOCK_ICON).map_err(|e| e.to_string())?;
    app.opener().reveal_item_in_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    let launch = app.autolaunch();
    if on { launch.enable() } else { launch.disable() }.map_err(|e| e.to_string())
}

// ---- window and tray --------------------------------------------------------

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

const TRAY_ID: &str = "ember";

struct TrayMenu {
    open: MenuItem<tauri::Wry>,
    toggle: CheckMenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

/// Puts the tray menu in the settings' language and state.
fn sync_tray(app: &AppHandle, s: &Settings) {
    let ru = s.lang == Lang::Ru;
    let t = |r: &'static str, e: &'static str| if ru { r } else { e };
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu.open.set_text(t("Открыть Ember", "Open Ember"));
        let _ = menu.toggle.set_text(t("Показывать статус", "Show status"));
        let _ = menu.toggle.set_checked(s.enabled);
        let _ = menu.quit.set_text(t("Выход", "Quit"));
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tip = match (s.enabled, ru) {
            (true, true) => "Ember: статус включён",
            (false, true) => "Ember: статус выключен",
            (true, false) => "Ember: status on",
            (false, false) => "Ember: status off",
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}

fn build_tray(app: &AppHandle, s: &Settings) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Ember", true, None::<&str>)?;
    let toggle = CheckMenuItem::with_id(app, "toggle", "Show status", true, s.enabled, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &PredefinedMenuItem::separator(app)?, &quit])?;
    app.manage(TrayMenu { open, toggle, quit });
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "toggle" => {
                let shared = app.state::<Shared>();
                let mut s = shared.settings.lock().unwrap().clone();
                s.enabled = !s.enabled;
                let _ = s.save(&shared.settings_path);
                sync_tray(app, &s);
                *shared.settings.lock().unwrap() = s;
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    sync_tray(app, s);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            fs::create_dir_all(&dir)?;
            let settings_path = dir.join("settings.json");
            let settings = Settings::load(&settings_path);
            let first_run = settings.client_id.trim().is_empty();
            build_tray(app.handle(), &settings)?;
            app.manage(Shared {
                settings: Mutex::new(settings),
                settings_path,
                catalog_path: dir.join("catalog.json"),
                live: Mutex::new(Live::default()),
            });

            for job in [watch, data, publish, memory::run] {
                let handle = app.handle().clone();
                thread::spawn(move || job(handle));
            }

            let minimized = std::env::args().any(|a| a == "--minimized");
            if first_run || !minimized {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides to the tray; the status keeps going.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_status,
            preview,
            preset,
            variables,
            open_url,
            save_icon,
            get_autostart,
            set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ember");
}
