//! Opt-in: the live match read from the game's memory, for players who accept
//! the risk. Off by default; the log stays the source of the phases either way.
//!
//! The reading is deadlock-reader's (deadlock-rs, MIT OR Apache-2.0): read-only
//! (`OpenProcess` + `ReadProcessMemory`, no writes, no injection, no hooks),
//! with field offsets taken from the game's own Source 2 schema so most
//! patches don't break it. When a patch does make it read numbers that only
//! look right, it says so and the numbers are dropped.

use std::{
    thread,
    time::{Duration, Instant},
};

use deadlock_reader::{
    Reader, Team,
    snapshot::{Context, LiveSnapshot, LiveState, Perspective},
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{Shared, now};

const TICK: Duration = Duration::from_secs(2);
const RETRY: Duration = Duration::from_secs(10);
/// Numbers older than this are not shown.
pub const FRESH: i64 = 8;

/// What the memory says about the match on screen.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveMatch {
    /// When it was read, unix seconds.
    pub read_at: i64,
    /// A real match (not the hideout, sandbox or a loading screen).
    pub in_match: bool,
    /// Watching someone else's match.
    pub spectating: bool,
    /// Whoever is on screen: you, or the player you watch.
    pub hero_id: Option<u32>,
    pub kills: Option<u32>,
    pub deaths: Option<u32>,
    pub assists: Option<u32>,
    pub souls: Option<u32>,
    pub level: Option<u32>,
    pub last_hits: Option<u32>,
    pub match_mode: Option<u32>,
    pub game_mode: Option<u32>,
    /// Street Brawl: round, your team's score, the other team's.
    pub brawl: Option<(i32, i32, i32)>,
    pub match_id: Option<u64>,
    /// Whether your team won, once the match is decided.
    pub won: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    /// "off", "waiting" (no game), "attached", "loading", "stale" (a patch broke
    /// the offsets) or "error".
    pub state: String,
    /// A code the window translates ("memory.admin"), or the reader's own error.
    pub message: Option<String>,
}

fn status(state: &str, message: Option<String>) -> MemoryStatus {
    MemoryStatus { state: state.into(), message }
}

pub fn run(app: AppHandle) {
    let shared = app.state::<Shared>();
    let mut reader: Option<Reader> = None;
    let mut retry_at = Instant::now();
    loop {
        thread::sleep(TICK);
        let enabled = shared.settings.lock().unwrap().memory;
        let running = shared.live.lock().unwrap().running;
        if !enabled || !running {
            reader = None;
            let mut live = shared.live.lock().unwrap();
            live.memory = None;
            live.memory_status = status(if enabled { "waiting" } else { "off" }, None);
            continue;
        }

        if reader.is_none() {
            if Instant::now() < retry_at {
                continue;
            }
            match Reader::attach() {
                Ok(r) => reader = Some(r),
                Err(e) => {
                    retry_at = Instant::now() + RETRY;
                    let message = if e.to_string().contains("os error 5") {
                        "memory.admin".to_string()
                    } else {
                        e.to_string()
                    };
                    shared.live.lock().unwrap().memory_status = status("error", Some(message));
                    continue;
                }
            }
        }
        let r = reader.as_ref().unwrap();

        let (found, state) = match r.live_state() {
            Ok(LiveState::Live(_)) if r.is_reading_corrupt_data() => (None, status("stale", None)),
            Ok(LiveState::Live(snap)) => (Some(read(&snap)), status("attached", None)),
            Ok(_) => (None, status("loading", None)),
            Err(e) => {
                reader = None;
                retry_at = Instant::now() + RETRY;
                (None, status("error", Some(e.to_string())))
            }
        };
        let mut live = shared.live.lock().unwrap();
        // The last numbers of a match outlive it: the post-game shows them
        // before deadlock-api has the match.
        if let Some(m) = found.as_ref().filter(|m| m.in_match && !m.spectating) {
            live.last_match = Some(m.clone());
        }
        live.memory = found;
        live.memory_status = state;
    }
}

fn read(snap: &LiveSnapshot) -> LiveMatch {
    let me = snap.current_player();
    let team = me.and_then(|p| p.team);
    let brawl = snap.street_brawl.as_ref().and_then(|b| {
        let (amber, sapphire) = (b.amber_score?, b.sapphire_score?);
        let (ours, theirs) = if team == Some(Team::SAPPHIRE) { (sapphire, amber) } else { (amber, sapphire) };
        Some((b.round?, ours, theirs))
    });
    LiveMatch {
        read_at: now(),
        in_match: snap.context == Context::Match,
        spectating: snap.perspective == Perspective::Spectating,
        hero_id: me.and_then(|p| p.hero_id).map(|h| h.0).filter(|&id| id > 0),
        kills: me.and_then(|p| p.kills),
        deaths: me.and_then(|p| p.deaths),
        assists: me.and_then(|p| p.assists),
        souls: me.and_then(|p| p.net_worth),
        level: me.and_then(|p| p.level),
        last_hits: me.and_then(|p| p.last_hits),
        match_mode: snap.match_mode_raw.filter(|&m| m > 0),
        game_mode: snap.game_mode_raw.filter(|&m| m > 0),
        brawl,
        match_id: snap.match_id.filter(|&id| id > 0),
        won: snap.winning_team.zip(team).map(|(winner, ours)| winner == ours),
    }
}
