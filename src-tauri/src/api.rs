//! deadlock-api.com: hero names and art, rank names and badges, and the
//! numbers of a finished match. Public data, no key needed.

use std::{fs, path::Path, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const BASE: &str = "https://api.deadlock-api.com/v1";
/// How long the hero and rank lists are trusted before being fetched again.
const CATALOG_TTL: i64 = 24 * 3600;
const CATALOG_VERSION: u32 = 2;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("Ember/", env!("CARGO_PKG_VERSION")))
        .build()
}

fn get(url: &str) -> Result<Value, String> {
    agent()
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hero {
    pub id: u32,
    pub name: String,
    /// The game's own hideout line, e.g. "Reading In The Hideout".
    pub hideout_line: Option<String>,
    pub icon: Option<String>,
    pub card: Option<String>,
    /// Model folder, the way the log names the hero ("familiar" for Rem).
    pub folder: Option<String>,
    pub class_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rank {
    pub tier: u32,
    pub name: String,
    pub image: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// Bumped when the cached shape changes, so an old cache is fetched again.
    #[serde(default)]
    pub version: u32,
    pub fetched: i64,
    pub heroes: Vec<Hero>,
    pub ranks: Vec<Rank>,
}

impl Catalog {
    /// The cached lists, fetched again once they're a day old. A failed fetch
    /// keeps the old lists.
    pub fn load(cache: &Path, now: i64) -> Catalog {
        let cached: Option<Catalog> = fs::read(cache).ok().and_then(|b| serde_json::from_slice(&b).ok());
        if let Some(c) = &cached {
            if c.version == CATALOG_VERSION && now - c.fetched < CATALOG_TTL && !c.heroes.is_empty() {
                return c.clone();
            }
        }
        match Self::fetch(now) {
            Ok(fresh) => {
                if let Ok(bytes) = serde_json::to_vec(&fresh) {
                    let _ = fs::write(cache, bytes);
                }
                fresh
            }
            Err(e) => {
                eprintln!("catalog: {e}");
                cached.unwrap_or_default()
            }
        }
    }

    fn fetch(now: i64) -> Result<Catalog, String> {
        let heroes = get(&format!("{BASE}/assets/heroes"))?;
        let heroes = heroes
            .as_array()
            .ok_or("heroes: not a list")?
            .iter()
            .filter(|h| h["player_selectable"].as_bool() == Some(true) && h["disabled"].as_bool() != Some(true))
            .filter_map(|h| {
                let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(String::from);
                Some(Hero {
                    id: h["id"].as_u64()? as u32,
                    name: text(&h["name"])?,
                    hideout_line: text(&h["hideout_rich_presence"]),
                    icon: text(&h["images"]["icon_image_small"]),
                    card: text(&h["images"]["icon_hero_card"]),
                    folder: h["models"]["hero_select"].as_str().and_then(model_folder).map(String::from),
                    class_name: text(&h["class_name"])?,
                })
            })
            .collect::<Vec<_>>();
        if heroes.is_empty() {
            return Err("heroes: empty list".into());
        }
        let ranks = get(&format!("{BASE}/assets/ranks"))?
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|r| {
                        Some(Rank {
                            tier: r["tier"].as_u64()? as u32,
                            name: r["name"].as_str()?.to_string(),
                            image: r["images"]["large"].as_str().map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Catalog { version: CATALOG_VERSION, fetched: now, heroes, ranks })
    }

    /// The hero for a log model folder.
    pub fn hero_by_folder(&self, folder: &str) -> Option<&Hero> {
        self.heroes
            .iter()
            .find(|h| h.folder.as_deref() == Some(folder))
            .or_else(|| self.heroes.iter().find(|h| h.class_name.strip_prefix("hero_") == Some(folder)))
    }

    pub fn hero_by_id(&self, id: u32) -> Option<&Hero> {
        self.heroes.iter().find(|h| h.id == id)
    }

    pub fn rank(&self, tier: u32) -> Option<&Rank> {
        self.ranks.iter().find(|r| r.tier == tier)
    }
}

/// `…/models/heroes_staging/digger/digger/hero_select.glb` → `digger`.
fn model_folder(url: &str) -> Option<&str> {
    let rest = &url[url.find("/models/heroes")? + "/models/heroes".len()..];
    let rest = &rest[rest.find('/')? + 1..];
    rest.split('/').next().filter(|s| !s.is_empty())
}

/// A ranked badge: tier and subrank (1..6), e.g. Ritualist 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Badge {
    pub tier: u32,
    pub subrank: u32,
}

pub fn player_badge(account: u32) -> Result<Option<Badge>, String> {
    let v = get(&format!("{BASE}/players/{account}/rank"))?;
    let tier = v["rank"].as_u64().unwrap_or(0) as u32;
    let subrank = v["subrank"].as_u64().unwrap_or(0) as u32;
    Ok((tier > 0).then_some(Badge { tier, subrank }))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchStats {
    pub match_id: u64,
    pub hero_id: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub souls: u32,
    pub last_hits: u32,
    pub level: u32,
    pub duration_s: u32,
    pub won: bool,
    pub game_mode: u32,
    pub match_mode: u32,
}

/// The player's line from their match history. The API keeps it up to date
/// only for accounts friends with its bots; for others it can lag by hours.
pub fn match_stats(account: u32, match_id: u64) -> Result<Option<MatchStats>, String> {
    let list = get(&format!("{BASE}/players/{account}/match-history"))?;
    let Some(m) = list
        .as_array()
        .and_then(|l| l.iter().find(|m| m["match_id"].as_u64() == Some(match_id)))
    else {
        return Ok(None);
    };
    let n = |k: &str| m[k].as_u64().unwrap_or(0) as u32;
    Ok(Some(MatchStats {
        match_id,
        hero_id: n("hero_id"),
        kills: n("player_kills"),
        deaths: n("player_deaths"),
        assists: n("player_assists"),
        souls: n("net_worth"),
        last_hits: n("last_hits"),
        level: n("hero_level"),
        duration_s: n("match_duration_s"),
        won: m["match_result"].as_u64().is_some() && m["match_result"] == m["player_team"],
        game_mode: n("game_mode"),
        match_mode: n("match_mode"),
    }))
}

/// The same numbers from the match's metadata, which the API fetches from
/// Steam when it doesn't have it yet. That costs: 3 requests an hour per IP,
/// and "Steam is busy" (503) is common, so call it rarely.
pub fn match_stats_from_metadata(account: u32, match_id: u64) -> Result<Option<MatchStats>, String> {
    let v = match agent().get(&format!("{BASE}/matches/{match_id}/metadata")).call() {
        Ok(r) => r.into_json::<Value>().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(404 | 503 | 504, _)) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let info = &v["match_info"];
    let Some(p) = info["players"]
        .as_array()
        .and_then(|l| l.iter().find(|p| p["account_id"].as_u64() == Some(account as u64)))
    else {
        return Ok(None);
    };
    let n = |v: &Value| v.as_u64().unwrap_or(0) as u32;
    Ok(Some(MatchStats {
        match_id,
        hero_id: n(&p["hero_id"]),
        kills: n(&p["kills"]),
        deaths: n(&p["deaths"]),
        assists: n(&p["assists"]),
        souls: n(&p["net_worth"]),
        last_hits: n(&p["last_hits"]),
        level: n(&p["level"]),
        duration_s: n(&info["duration_s"]),
        won: info["winning_team"].as_u64().is_some() && info["winning_team"] == p["team"],
        game_mode: n(&info["game_mode"]),
        match_mode: n(&info["match_mode"]),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_from_model_url() {
        let url = "https://x/assets-api-res/models/heroes_staging/yamato_v2/yamato/hero_select.glb";
        assert_eq!(model_folder(url), Some("yamato_v2"));
        let url = "https://x/assets-api-res/models/heroes_wip/familiar/familiar_wip/hero_select.glb";
        assert_eq!(model_folder(url), Some("familiar"));
    }
}
