//! What the user set up: their Discord application and a status per game
//! phase. Stored as settings.json in the app's data folder.

use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::log::Phase;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Image {
    /// The Discord application's own icon.
    App,
    /// Deadlock's icon from Steam.
    Deadlock,
    HeroIcon,
    HeroCard,
    Rank,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Timer {
    None,
    /// Since this phase began.
    State,
    /// Since the fight began.
    Match,
    /// Since the search began.
    Queue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Lang {
    Ru,
    En,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StateCfg {
    pub enabled: bool,
    pub details: String,
    pub state: String,
    pub image: Image,
    pub small: Image,
    pub timer: Timer,
}

impl Default for StateCfg {
    fn default() -> Self {
        Self {
            enabled: true,
            details: String::new(),
            state: String::new(),
            image: Image::HeroIcon,
            small: Image::Rank,
            timer: Timer::State,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ButtonCfg {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub enabled: bool,
    pub client_id: String,
    /// The Discord app has Deadlock's icon as a Rich Presence asset named
    /// `deadlock`, so the sharp one is used instead of Steam's 32 px icon.
    pub deadlock_asset: bool,
    /// The Deadlock folder; empty to find it through Steam.
    pub deadlock_dir: String,
    /// How long the post-game status stays after returning to the hideout.
    pub post_game_minutes: u32,
    /// Language of the window, the tray and the words Ember fills in itself
    /// ({result}, {mode}).
    pub lang: Lang,
    /// Live match numbers from the game's memory (see memory.rs). The user
    /// turns it on after reading the risks.
    pub memory: bool,
    pub states: BTreeMap<Phase, StateCfg>,
    pub buttons: Vec<ButtonCfg>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: String::new(),
            deadlock_asset: false,
            deadlock_dir: String::new(),
            post_game_minutes: 3,
            lang: Lang::En,
            memory: false,
            states: preset(Lang::En),
            buttons: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        let mut s: Settings = fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        // States added in a later version get their preset.
        for (phase, cfg) in preset(s.lang) {
            s.states.entry(phase).or_insert(cfg);
        }
        s.states.retain(|p, _| Phase::SHOWN.contains(p));
        s.buttons.truncate(2);
        s
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
}

/// The built-in wording for every phase.
pub fn preset(lang: Lang) -> BTreeMap<Phase, StateCfg> {
    let ru = lang == Lang::Ru;
    let t = |r: &str, e: &str| if ru { r.to_string() } else { e.to_string() };
    let cfg = |details: String, state: String, image: Image, timer: Timer| StateCfg {
        enabled: true,
        details,
        state,
        image,
        small: Image::Rank,
        timer,
    };
    BTreeMap::from([
        (
            Phase::Launching,
            cfg(t("Запускает игру", "Starting the game"), String::new(), Image::Deadlock, Timer::State),
        ),
        (
            Phase::Hideout,
            cfg(
                t("Отдыхает в хайдауте", "Relaxing in the Hideout"),
                t("Играет за {hero} || В главном меню", "Hanging out as {hero} || In the main menu"),
                Image::HeroIcon,
                Timer::State,
            ),
        ),
        (
            Phase::Queue,
            cfg(
                t("Ищет матч", "Finding a match"),
                t("{rank} || В очереди", "{rank} || In the queue"),
                Image::Deadlock,
                Timer::Queue,
            ),
        ),
        (
            Phase::Found,
            cfg(t("Матч найден", "Match found"), t("Подключается…", "Connecting…"), Image::Deadlock, Timer::State),
        ),
        (
            Phase::Loading,
            cfg(
                t("Матч начинается", "Match is starting"),
                t("За {hero} || Выбор героев", "As {hero} || Hero reveal"),
                Image::HeroIcon,
                Timer::State,
            ),
        ),
        (
            Phase::Match,
            cfg(
                t("В матче за {hero} || В матче", "In a match as {hero} || In a match"),
                // The numbers come only from memory; the rank is the fallback.
                t("{kda} · {souls} душ || {rank}", "{kda} · {souls} souls || {rank}"),
                Image::HeroIcon,
                Timer::Match,
            ),
        ),
        (
            Phase::Spectating,
            cfg(
                t("Смотрит матч", "Spectating a match"),
                t("Следит за {hero} || {mode}", "Watching {hero} || {mode}"),
                Image::HeroIcon,
                Timer::Match,
            ),
        ),
        (
            Phase::PostGame,
            cfg(
                t("{result} за {hero} || Матч завершён", "{result} as {hero} || Match over"),
                t(
                    "{kda} · {souls} душ || Матч длился {duration}",
                    "{kda} · {souls} souls || Match lasted {duration}",
                ),
                Image::HeroCard,
                Timer::None,
            ),
        ),
        (
            Phase::Sandbox,
            cfg(
                t("Тренируется с ботами", "Practicing with bots"),
                t("За {hero}", "As {hero}"),
                Image::HeroIcon,
                Timer::State,
            ),
        ),
    ])
}
