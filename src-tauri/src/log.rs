//! Deadlock's console.log (written with -condebug) read line by line into
//! what the player is doing. Nothing here touches the game itself: these are
//! the lines the game prints anyway.
//!
//! The lines it relies on:
//! - `[Client] Map: "dl_hideout"` / `"start"`: the hideout or a match map;
//! - `[Client] CL:  Connected to 'loopback:1'`: a local server, so bots;
//! - `Send msg 9142 (k_EMsgClientToGCPartySetReadyState)`: queue on or off;
//! - `Lobby … for Match 123807324 created` and `OnMatchFormed`: match found;
//! - `OnGameStateChanged: GameInProgress (7)`, `PostGame (8)`: match phases;
//! - ` VMDL Camera Pose Success! "models/heroes_wip/werewolf/…"`: the camera
//!   of a hero pawn loaded. In a match the first one is your own hero; later
//!   ones may be whoever you watch after dying.

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// Deadlock isn't running.
    Closed,
    /// Running, but not in the hideout yet.
    Launching,
    Hideout,
    /// Looking for a match.
    Queue,
    /// A match was found and the game is connecting to it.
    Found,
    /// On the match map before the fight: hero reveal, loading, pre-game.
    Loading,
    Match,
    PostGame,
    /// A match on a local server: bots or a sandbox.
    Sandbox,
    /// Watching someone else's match. Only the memory tells this apart.
    Spectating,
}

impl Phase {
    /// The phases a status can be set up for.
    pub const SHOWN: [Phase; 9] = [
        Phase::Launching,
        Phase::Hideout,
        Phase::Queue,
        Phase::Found,
        Phase::Loading,
        Phase::Match,
        Phase::PostGame,
        Phase::Sandbox,
        Phase::Spectating,
    ];
}

/// The match that ended last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finished {
    pub match_id: Option<u64>,
    /// Model folder of the hero played, as in [`Tracker::hero`].
    pub hero: Option<String>,
    /// When the fight began, if the log showed it.
    pub started: Option<i64>,
    pub ended: i64,
}

const HIDEOUT: &str = "dl_hideout";

#[derive(Clone, Debug)]
pub struct Tracker {
    pub phase: Phase,
    /// When the phase began, unix seconds.
    pub since: i64,
    /// Model folder of the hero ("werewolf", "familiar"), see `api::Catalog`.
    pub hero: Option<String>,
    pub match_id: Option<u64>,
    /// When the fight began (GameInProgress).
    pub match_start: Option<i64>,
    pub queue_start: Option<i64>,
    pub finished: Option<Finished>,
    /// Lines older than this are left over from the previous run.
    min_time: i64,
    map: Option<String>,
    local_server: bool,
    /// In a match, the hero is settled by the first camera after loading.
    hero_locked: bool,
    /// Time of the last timestamped line; continuation lines have none.
    now: i64,
    stamp_cache: (String, i64),
}

impl Tracker {
    /// A tracker for a game started at `started` (unix seconds).
    pub fn new(started: i64) -> Self {
        Self {
            phase: Phase::Launching,
            since: started,
            hero: None,
            match_id: None,
            match_start: None,
            queue_start: None,
            finished: None,
            min_time: started - 120,
            map: None,
            local_server: false,
            hero_locked: false,
            now: started,
            stamp_cache: (String::new(), 0),
        }
    }

    pub fn feed(&mut self, line: &str) {
        let line = line.trim_start_matches('\u{feff}').trim_end_matches(['\r', '\n']);
        let text = match self.split_stamp(line) {
            Some((t, rest)) => {
                self.now = t;
                rest
            }
            None => line,
        };
        if self.now < self.min_time {
            return;
        }
        let t = self.now;

        if let Some(folder) = camera_hero(text) {
            self.on_camera(folder);
        } else if let Some(map) = between(text, "[Client] Map: \"", "\"") {
            self.on_map(map, t);
        } else if let Some(addr) = between(text, "[Client] CL:  Connected to '", "'") {
            self.local_server = addr.starts_with("loopback") || addr.starts_with("127.0.0.1");
        } else if text.contains("Send msg") && text.contains("(k_EMsgClientToGCPartySetReadyState)") {
            self.on_ready_toggle(t);
        } else if let Some(id) = between(text, "Lobby ", " created").and_then(|s| s.split(" for Match ").nth(1)) {
            self.match_id = id.trim().parse().ok().or(self.match_id);
        } else if text.contains("CCitadel_PlayController::OnMatchFormed") {
            self.queue_start = None;
            if !matches!(self.phase, Phase::Loading | Phase::Match) {
                self.set(Phase::Found, t);
            }
        } else if let Some(state) = text.strip_prefix("OnGameStateChanged: ") {
            let name = state.split_whitespace().next().unwrap_or("");
            self.on_game_state(name, t);
        }
    }

    fn set(&mut self, phase: Phase, t: i64) {
        if self.phase != phase {
            self.phase = phase;
            self.since = t;
        }
    }

    fn on_map(&mut self, map: &str, t: i64) {
        if map == "<empty>" {
            return;
        }
        self.map = Some(map.to_string());
        self.hero_locked = false;
        if map == HIDEOUT {
            if self.phase == Phase::Queue {
                return;
            }
            // A finished match is kept in `finished`; an abandoned one is gone.
            self.match_id = None;
            self.match_start = None;
            self.queue_start = None;
            self.set(Phase::Hideout, t);
            return;
        }
        // A match map: the hero is told by the first camera that loads.
        self.hero = None;
        self.queue_start = None;
        if self.local_server {
            self.match_id = None;
            self.match_start = None;
            self.set(Phase::Sandbox, t);
        } else if !matches!(self.phase, Phase::Loading | Phase::Match) {
            self.match_start = None;
            self.set(Phase::Loading, t);
        }
    }

    fn on_camera(&mut self, folder: &str) {
        let in_match = self.map.as_deref().is_some_and(|m| m != HIDEOUT);
        if in_match {
            if self.hero_locked {
                return;
            }
            self.hero_locked = true;
        }
        self.hero = Some(folder.to_string());
    }

    /// The same message starts and stops the search.
    fn on_ready_toggle(&mut self, t: i64) {
        match self.phase {
            Phase::Hideout | Phase::Launching | Phase::PostGame => {
                self.queue_start = Some(t);
                self.set(Phase::Queue, t);
            }
            Phase::Queue => {
                self.queue_start = None;
                self.set(Phase::Hideout, t);
            }
            _ => {}
        }
    }

    fn on_game_state(&mut self, name: &str, t: i64) {
        // The hideout is a "game" too and goes through the same states.
        if self.map.as_deref().is_none_or(|m| m == HIDEOUT) {
            return;
        }
        if self.local_server {
            if name == "GameInProgress" {
                self.match_start.get_or_insert(t);
            }
            return;
        }
        match name {
            "WaitingForPlayersToJoin" | "HeroSelection" | "MatchIntro" | "WaitForMapToLoad" | "PreGameWait" => {
                if self.phase != Phase::Match {
                    self.set(Phase::Loading, t);
                }
            }
            "GameInProgress" => {
                self.match_start.get_or_insert(t);
                self.set(Phase::Match, t);
            }
            "PostGame" => {
                self.finished = Some(Finished {
                    match_id: self.match_id,
                    hero: self.hero.clone(),
                    started: self.match_start,
                    ended: t,
                });
                self.set(Phase::PostGame, t);
            }
            "Abandoned" => {
                self.finished = None;
                self.set(Phase::Loading, t);
            }
            _ => {}
        }
    }

    /// `10/24 21:22:15 rest` → unix seconds and the rest.
    fn split_stamp<'a>(&mut self, line: &'a str) -> Option<(i64, &'a str)> {
        let stamp = line.get(..14)?;
        let rest = line.get(15..).unwrap_or("");
        let b = stamp.as_bytes();
        if b[2] != b'/' || b[5] != b' ' || b[8] != b':' || b[11] != b':' {
            return None;
        }
        if self.stamp_cache.0 == stamp {
            return Some((self.stamp_cache.1, rest));
        }
        let num = |r: std::ops::Range<usize>| stamp.get(r)?.parse::<u32>().ok();
        let t = local_unix(num(0..2)?, num(3..5)?, num(6..8)?, num(9..11)?, num(12..14)?)?;
        self.stamp_cache = (stamp.to_string(), t);
        Some((t, rest))
    }
}

/// The log has no year: take this one, or the last one for a December log read in January.
fn local_unix(month: u32, day: u32, h: u32, m: u32, s: u32) -> Option<i64> {
    let now = Local::now();
    let at = |year: i32| {
        let naive = NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(h, m, s)?;
        Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp())
    };
    let t = at(now.year())?;
    if t > now.timestamp() + 86_400 { at(now.year() - 1) } else { Some(t) }
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let len = text[from..].find(end)?;
    Some(&text[from..from + len])
}

/// `VMDL Camera Pose Success! "models/heroes_wip/werewolf/…"` → `werewolf`.
fn camera_hero(text: &str) -> Option<&str> {
    let path = between(text, "VMDL Camera Pose Success! \"", "\"")?;
    let rest = path.strip_prefix("models/heroes")?;
    let rest = &rest[rest.find('/')? + 1..];
    rest.split('/').next().filter(|f| !f.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lines: &[&str]) -> Tracker {
        let mut t = Tracker::new(0);
        for l in lines {
            t.feed(l);
        }
        t
    }

    const BOOT: &[&str] = &[
        "\u{feff}10/24 20:55:53 [CommandLine] -steam -console -condebug",
        "10/24 20:56:11 [Client] CL:  Connected to 'loopback:1'",
        "10/24 20:56:11 [Client] Map: \"<empty>\"",
        "10/24 21:00:15 [Client] CL:  Connected to '=[A:1:3147002895:52087]'",
        "10/24 21:00:15 [Client] Map: \"dl_hideout\"",
        "10/24 21:00:57 OnGameStateChanged: GameInProgress (7)",
        " VMDL Camera Pose Success! \"models/heroes_wip/familiar/familiar_wip.vmdl\" has loaded camera poses",
    ];

    #[test]
    fn hideout_with_hero() {
        let t = run(BOOT);
        assert_eq!(t.phase, Phase::Hideout);
        assert_eq!(t.hero.as_deref(), Some("familiar"));
    }

    #[test]
    fn hero_changes_in_the_hideout() {
        let mut lines = BOOT.to_vec();
        lines.push(" VMDL Camera Pose Success! \"models/heroes_staging/yamato_v2/yamato.vmdl\" has loaded");
        assert_eq!(run(&lines).hero.as_deref(), Some("yamato_v2"));
    }

    #[test]
    fn a_whole_match() {
        let mut lines = BOOT.to_vec();
        lines.extend([
            "10/24 21:18:37 [GCClient] Send msg 9142 (k_EMsgClientToGCPartySetReadyState), 60 bytes",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Queue);
        assert!(t.queue_start.is_some());

        lines.extend([
            "10/24 21:22:15 Lobby 102135642515365550 for Match 123807324 created",
            "10/24 21:22:15 [Citadel Play Controller] CCitadel_PlayController::OnMatchFormed",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Found);
        assert_eq!(t.match_id, Some(123807324));
        assert_eq!(t.queue_start, None);

        lines.extend([
            "10/24 21:22:16 [Client] CL:  Connected to '=[A:1:1302248464:52088]'",
            "10/24 21:22:16 [Client] Map: \"start\"",
            "10/24 21:23:04 OnGameStateChanged: MatchIntro (4)",
            "10/24 21:23:11 OnGameStateChanged: PreGameWait (6)",
            " VMDL Camera Pose Success! \"models/heroes_wip/werewolf/werewolf.vmdl\" has loaded camera poses",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Loading);
        assert_eq!(t.hero.as_deref(), Some("werewolf"));

        lines.extend([
            "10/24 21:23:39 OnGameStateChanged: GameInProgress (7)",
            // Watching the killer after dying must not change the hero.
            " VMDL Camera Pose Success! \"models/heroes_wip/ivy/ivy.vmdl\" has loaded camera poses",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Match);
        assert_eq!(t.hero.as_deref(), Some("werewolf"));
        let start = t.match_start.unwrap();
        assert_eq!(t.since, start);

        lines.extend([
            "10/24 21:56:12 OnGameStateChanged: PostGame (8)",
            "10/24 21:56:22 OnGameStateChanged: End (11)",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::PostGame);
        let done = t.finished.clone().unwrap();
        assert_eq!(done.match_id, Some(123807324));
        assert_eq!(done.hero.as_deref(), Some("werewolf"));
        assert_eq!(done.ended - start, 32 * 60 + 33);

        lines.extend([
            "10/24 21:56:25 [Client] CL:  Connected to 'loopback:1'",
            "10/24 21:56:25 [Client] Map: \"<empty>\"",
            "10/24 21:56:25 [Client] CL:  Connected to '=[A:1:4008702989:52088]'",
            "10/24 21:56:26 [Client] Map: \"dl_hideout\"",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Hideout);
        assert!(t.finished.is_some());
        assert_eq!(t.match_id, None);
    }

    #[test]
    fn queue_can_be_cancelled() {
        let mut lines = BOOT.to_vec();
        lines.push("10/24 21:18:37 [GCClient] Send msg 9142 (k_EMsgClientToGCPartySetReadyState), 60 bytes");
        lines.push("10/24 21:19:37 [GCClient] Send msg 9142 (k_EMsgClientToGCPartySetReadyState), 66 bytes");
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Hideout);
        assert_eq!(t.queue_start, None);
    }

    #[test]
    fn bots_on_a_local_server() {
        let mut lines = BOOT.to_vec();
        lines.extend([
            "10/24 21:30:00 [Client] CL:  Connected to '127.0.0.1:27015'",
            "10/24 21:30:01 [Client] Map: \"start\"",
            "10/24 21:30:05 OnGameStateChanged: GameInProgress (7)",
        ]);
        let t = run(&lines);
        assert_eq!(t.phase, Phase::Sandbox);
        assert!(t.match_start.is_some());
    }

    #[test]
    fn old_lines_are_skipped() {
        let mut t = Tracker::new(i64::MAX / 2);
        for l in BOOT {
            t.feed(l);
        }
        assert_eq!(t.phase, Phase::Launching);
    }

    #[test]
    fn real_log_if_present() {
        let path = r"D:\SteamLibrary\steamapps\common\Deadlock\game\citadel\console.log";
        let Ok(bytes) = std::fs::read(path) else { return };
        let text = String::from_utf8_lossy(&bytes);
        let mut t = Tracker::new(0);
        let mut last = (t.phase, t.hero.clone());
        for l in text.lines() {
            t.feed(l);
            if (t.phase, t.hero.clone()) != last {
                last = (t.phase, t.hero.clone());
                let at = chrono::DateTime::from_timestamp(t.now, 0).unwrap().with_timezone(&Local);
                eprintln!("{} {:?} hero={:?} match={:?}", at.format("%H:%M:%S"), t.phase, t.hero, t.match_id);
            }
        }
    }
}
