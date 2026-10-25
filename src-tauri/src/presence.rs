//! Turns what's known about the game into the Discord activity, by the
//! user's templates.
//!
//! A line is a template like `{result} as {hero}`. `||` separates fallbacks:
//! the first one whose variables are all known is used, so
//! `{kda} · {souls} souls || Waiting for match stats` shows the second part
//! until the numbers arrive. A line with nothing left is not sent.

use std::collections::HashMap;

use serde::Serialize;

use crate::{
    api::MatchStats,
    log::Phase,
    memory::LiveMatch,
    settings::{ButtonCfg, Image, Lang, StateCfg, Timer},
};

/// Variables a template can use, in the order the editor lists them.
pub const VARIABLES: [&str; 18] = [
    "hero", "hero_line", "rank", "result", "kda", "kills", "deaths", "assists", "souls", "last_hits",
    "level", "duration", "mode", "round", "score", "match_id", "account_id", "steam_id",
];

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroFacts {
    pub name: String,
    pub line: Option<String>,
    pub icon: Option<String>,
    pub card: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankFacts {
    pub name: String,
    pub image: Option<String>,
}

/// Everything known at the moment, gathered from the log and the API.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    /// The phase the status is shown for (post-game lingers in the hideout).
    pub phase: Phase,
    pub since: i64,
    pub match_start: Option<i64>,
    pub queue_start: Option<i64>,
    pub hero: Option<HeroFacts>,
    pub rank: Option<RankFacts>,
    /// The match whose numbers are shown: the one just finished.
    pub stats: Option<MatchStats>,
    /// The match being played, from memory, when that's turned on.
    pub live: Option<LiveMatch>,
    /// How long the finished match lasted, by the log.
    pub played_s: Option<u32>,
    pub match_id: Option<u64>,
    pub account_id: Option<u32>,
    pub steam_id: Option<u64>,
    /// Deadlock's icon, see `steam::deadlock_icon_url`.
    pub game_icon: Option<String>,
}

impl Default for Facts {
    fn default() -> Self {
        Self {
            phase: Phase::Closed,
            since: 0,
            match_start: None,
            queue_start: None,
            hero: None,
            rank: None,
            stats: None,
            live: None,
            played_s: None,
            match_id: None,
            account_id: None,
            steam_id: None,
            game_icon: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Timestamps {
    /// Unix milliseconds.
    pub start: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Assets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Button {
    pub label: String,
    pub url: String,
}

/// A Discord activity, in the shape SET_ACTIVITY takes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<Button>>,
}

pub fn variables(f: &Facts, lang: Lang) -> HashMap<&'static str, String> {
    let ru = lang == Lang::Ru;
    let mut v = HashMap::new();
    if let Some(h) = &f.hero {
        v.insert("hero", h.name.clone());
        if let Some(line) = &h.line {
            v.insert("hero_line", line.clone());
        }
    }
    if let Some(r) = &f.rank {
        v.insert("rank", r.name.clone());
    }
    if let Some(id) = f.match_id.or(f.stats.as_ref().map(|s| s.match_id)) {
        v.insert("match_id", id.to_string());
    }
    if let Some(id) = f.account_id {
        v.insert("account_id", id.to_string());
    }
    if let Some(id) = f.steam_id {
        v.insert("steam_id", id.to_string());
    }
    if let Some(s) = &f.stats {
        v.insert("result", result_word(s.won, ru).into());
        v.insert("kda", format!("{}/{}/{}", s.kills, s.deaths, s.assists));
        v.insert("kills", s.kills.to_string());
        v.insert("deaths", s.deaths.to_string());
        v.insert("assists", s.assists.to_string());
        v.insert("souls", short_number(s.souls));
        v.insert("last_hits", s.last_hits.to_string());
        v.insert("level", s.level.to_string());
        v.insert("mode", mode_name(s.game_mode, s.match_mode, ru).into());
    }
    if let Some(d) = f.stats.as_ref().map(|s| s.duration_s).filter(|&d| d > 0).or(f.played_s) {
        v.insert("duration", format!("{}:{:02}", d / 60, d % 60));
    }
    // Live numbers win over the last match's while a match is on screen.
    if let Some(l) = &f.live {
        let mut put = |k: &'static str, n: Option<u32>| {
            if let Some(n) = n {
                v.insert(k, n.to_string());
            }
        };
        put("kills", l.kills);
        put("deaths", l.deaths);
        put("assists", l.assists);
        put("last_hits", l.last_hits);
        put("level", l.level);
        if let (Some(k), Some(d), Some(a)) = (l.kills, l.deaths, l.assists) {
            v.insert("kda", format!("{k}/{d}/{a}"));
        }
        if let Some(s) = l.souls {
            v.insert("souls", short_number(s));
        }
        if l.match_mode.is_some() || l.game_mode.is_some() {
            v.insert("mode", mode_name(l.game_mode.unwrap_or(0), l.match_mode.unwrap_or(0), ru).into());
        }
        if let Some(won) = l.won {
            v.insert("result", result_word(won, ru).into());
        }
        if let Some((round, ours, theirs)) = l.brawl {
            v.insert("round", round.to_string());
            v.insert("score", format!("{ours}:{theirs}"));
        }
    }
    v
}

fn result_word(won: bool, ru: bool) -> &'static str {
    match (won, ru) {
        (true, true) => "Победа",
        (false, true) => "Поражение",
        (true, false) => "Won",
        (false, false) => "Lost",
    }
}

/// 22196 → "22.2k".
fn short_number(n: u32) -> String {
    if n < 1000 { n.to_string() } else { format!("{:.1}k", n as f64 / 1000.0) }
}

/// deadlock-api's game_mode (4 is Street Brawl) and match_mode (4 is ranked).
fn mode_name(game_mode: u32, match_mode: u32, ru: bool) -> &'static str {
    match (game_mode, match_mode, ru) {
        (4, _, _) => "Street Brawl",
        (_, 4, true) => "Рейтинговый",
        (_, 4, false) => "Ranked",
        (_, 2, true) => "Приватное лобби",
        (_, 2, false) => "Private lobby",
        (_, 3, true) => "С ботами",
        (_, 3, false) => "Bot match",
        (_, _, true) => "Обычный",
        (_, _, false) => "Unranked",
    }
}

/// One line: the first fallback whose variables are all known.
pub fn render_line(template: &str, vars: &HashMap<&'static str, String>) -> Option<String> {
    template.split("||").find_map(|alt| {
        let filled = fill(alt, vars)?;
        let filled = filled.trim();
        (!filled.is_empty()).then(|| fit(filled, 128))
    })
}

/// Substitutes `{name}`; `None` if a known variable has no value. Unknown
/// names stay as typed, so a typo shows up in the preview.
fn fill(text: &str, vars: &HashMap<&'static str, String>) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return Some(out);
        };
        let name = &after[..close];
        if VARIABLES.contains(&name) {
            out.push_str(vars.get(name)?);
        } else {
            out.push_str(&rest[open..open + close + 2]);
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Discord takes 2..=max characters in a field.
fn fit(s: &str, max: usize) -> String {
    let mut s: String = s.chars().take(max).collect();
    if s.chars().count() < 2 {
        s.push(' ');
    }
    s
}

fn image(kind: Image, f: &Facts) -> Option<(String, Option<String>)> {
    let hero_text = || f.hero.as_ref().map(|h| h.name.clone());
    match kind {
        Image::HeroIcon => Some((f.hero.as_ref()?.icon.clone()?, hero_text())),
        Image::HeroCard => Some((f.hero.as_ref()?.card.clone()?, hero_text())),
        Image::Rank => {
            let r = f.rank.as_ref()?;
            Some((r.image.clone()?, Some(r.name.clone())))
        }
        Image::Deadlock => Some((f.game_icon.clone()?, Some("Deadlock".into()))),
        Image::App | Image::None => None,
    }
}

/// The activity for the current phase, or `None` to show nothing.
pub fn build(cfg: &StateCfg, buttons: &[ButtonCfg], f: &Facts, lang: Lang) -> Option<Activity> {
    if !cfg.enabled || f.phase == Phase::Closed {
        return None;
    }
    let vars = variables(f, lang);
    let large = image(cfg.image, f);
    let small = image(cfg.small, f).filter(|s| large.as_ref().is_none_or(|l| l.0 != s.0));
    let assets = Assets {
        large_text: large.as_ref().and_then(|l| l.1.clone()).map(|t| fit(&t, 128)),
        large_image: large.map(|l| l.0),
        small_text: small.as_ref().and_then(|s| s.1.clone()).map(|t| fit(&t, 128)),
        small_image: small.map(|s| s.0),
    };
    let start = match cfg.timer {
        Timer::None => None,
        Timer::State => Some(f.since),
        Timer::Match => Some(f.match_start.unwrap_or(f.since)),
        Timer::Queue => Some(f.queue_start.unwrap_or(f.since)),
    };
    let buttons: Vec<Button> = buttons
        .iter()
        .filter_map(|b| {
            let label = render_line(&b.label, &vars)?;
            let url = fill(b.url.trim(), &vars)?;
            (url.starts_with("https://") || url.starts_with("http://")).then(|| Button {
                label: label.chars().take(32).collect(),
                url: url.chars().take(512).collect(),
            })
        })
        .take(2)
        .collect();
    Some(Activity {
        kind: 0,
        details: render_line(&cfg.details, &vars),
        state: render_line(&cfg.state, &vars),
        timestamps: start.filter(|&s| s > 0).map(|s| Timestamps { start: s * 1000 }),
        assets: (assets != Assets::default()).then_some(assets),
        buttons: (!buttons.is_empty()).then_some(buttons),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&'static str, &str)]) -> HashMap<&'static str, String> {
        pairs.iter().map(|(k, v)| (*k, v.to_string())).collect()
    }

    #[test]
    fn fallbacks() {
        let v = vars(&[("hero", "Silver")]);
        assert_eq!(render_line("{result} as {hero} || Match over", &v).as_deref(), Some("Match over"));
        assert_eq!(render_line("As {hero} || Hero reveal", &v).as_deref(), Some("As Silver"));
        assert_eq!(render_line("{rank}", &v), None);
        assert_eq!(render_line("", &v), None);
    }

    #[test]
    fn unknown_names_stay() {
        let v = vars(&[]);
        assert_eq!(render_line("{heroo} hi", &v).as_deref(), Some("{heroo} hi"));
        assert_eq!(render_line("a { b", &v).as_deref(), Some("a { b"));
    }

    #[test]
    fn post_game_numbers() {
        let f = Facts {
            phase: Phase::PostGame,
            stats: Some(MatchStats {
                match_id: 1,
                hero_id: 80,
                kills: 14,
                deaths: 4,
                assists: 9,
                souls: 31_240,
                last_hits: 100,
                level: 26,
                duration_s: 1928,
                won: true,
                game_mode: 1,
                match_mode: 4,
            }),
            hero: Some(HeroFacts { name: "Haze".into(), ..Default::default() }),
            ..Default::default()
        };
        let v = variables(&f, Lang::En);
        assert_eq!(render_line("{result} as {hero}", &v).as_deref(), Some("Won as Haze"));
        assert_eq!(render_line("{kda} · {souls} souls", &v).as_deref(), Some("14/4/9 · 31.2k souls"));
        assert_eq!(v["duration"], "32:08");
        assert_eq!(v["mode"], "Ranked");
    }

    #[test]
    fn live_numbers_from_memory() {
        let f = Facts {
            phase: Phase::Match,
            live: Some(LiveMatch {
                kills: Some(7),
                deaths: Some(2),
                assists: Some(11),
                souls: Some(24_130),
                match_mode: Some(4),
                brawl: Some((3, 2, 1)),
                ..Default::default()
            }),
            ..Default::default()
        };
        let v = variables(&f, Lang::Ru);
        assert_eq!(render_line("{kda} · {souls} душ || {rank}", &v).as_deref(), Some("7/2/11 · 24.1k душ"));
        assert_eq!(v["mode"], "Рейтинговый");
        assert_eq!(render_line("Раунд {round}, {score}", &v).as_deref(), Some("Раунд 3, 2:1"));
        // The post-game from memory, before deadlock-api has the match.
        let f = Facts {
            phase: Phase::PostGame,
            live: Some(LiveMatch { kills: Some(9), deaths: Some(3), assists: Some(6), won: Some(true), ..Default::default() }),
            hero: Some(HeroFacts { name: "Rem".into(), ..Default::default() }),
            ..Default::default()
        };
        let v = variables(&f, Lang::Ru);
        assert_eq!(render_line("{result} за {hero} || Матч завершён", &v).as_deref(), Some("Победа за Rem"));
        assert_eq!(v["kda"], "9/3/6");
        // Without memory the same line falls back to the rank.
        let v = variables(&Facts { rank: Some(RankFacts { name: "Emissary 3".into(), image: None }), ..Default::default() }, Lang::Ru);
        assert_eq!(render_line("{kda} · {souls} душ || {rank}", &v).as_deref(), Some("Emissary 3"));
    }

    #[test]
    fn closed_game_shows_nothing() {
        let cfg = StateCfg::default();
        assert_eq!(build(&cfg, &[], &Facts::default(), Lang::Ru), None);
    }
}
