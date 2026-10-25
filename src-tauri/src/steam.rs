//! Where Deadlock is installed, who plays it and whether it's running.

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[cfg(windows)]
use windows::{
    Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::{
            Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS},
            Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW},
            Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        },
    },
    core::w,
};

const GAME_EXE: &str = "deadlock.exe";
/// SteamID64 of account id 0.
const STEAM_ID_BASE: u64 = 76_561_197_960_265_728;

/// `…/steamapps/common/Deadlock/game/citadel`, where console.log is written.
pub fn citadel_dir(custom: &str) -> Option<PathBuf> {
    let custom = custom.trim();
    if !custom.is_empty() {
        let root = PathBuf::from(custom);
        return [root.join("game").join("citadel"), root.clone()]
            .into_iter()
            .find(|p| p.join("gameinfo.gi").exists());
    }
    libraries()
        .into_iter()
        .map(|lib| lib.join("steamapps/common/Deadlock/game/citadel"))
        .find(|p| p.join("gameinfo.gi").exists())
}

/// Steam library folders, the main one first.
fn libraries() -> Vec<PathBuf> {
    let mut steam = Vec::new();
    if let Some(path) = steam_path() {
        steam.push(path);
    }
    steam.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    let mut out = Vec::new();
    for root in steam {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(vdf) {
            for line in text.lines() {
                let mut parts = line.split('"').filter(|s| !s.trim().is_empty());
                if parts.next() == Some("path") {
                    if let Some(p) = parts.next() {
                        out.push(PathBuf::from(p.replace(r"\\", r"\")));
                    }
                }
            }
        }
        out.push(root);
    }
    out.dedup();
    out
}

#[cfg(windows)]
fn steam_path() -> Option<PathBuf> {
    let mut buf = [0u16; 512];
    let mut len = (buf.len() * 2) as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Valve\Steam"),
            w!("SteamPath"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut len),
        )
        .ok()
        .ok()?;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..end])))
}

#[cfg(not(windows))]
fn steam_path() -> Option<PathBuf> {
    None
}

/// The account that played last: the game keeps `account_<id>.stats` next to the log.
pub fn account_id(citadel: &Path) -> Option<u32> {
    fs::read_dir(citadel)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_prefix("account_")?.strip_suffix(".stats")?.parse().ok()?;
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, id))
        })
        .max()
        .map(|(_, id)| id)
}

/// Deadlock's own icon on Steam's CDN. Its name is a hash that changes when
/// Valve updates the icon; the Steam client keeps the current one in its
/// library cache, so that one is preferred.
pub fn deadlock_icon_url() -> String {
    const APP: &str = "1422450";
    const KNOWN: &str = "f6da1420a173324d49bcd470fa3eee781ad0fa5e";
    let cached = libraries().into_iter().find_map(|root| {
        fs::read_dir(root.join("appcache/librarycache").join(APP))
            .ok()?
            .flatten()
            .find_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let hash = name.strip_suffix(".jpg")?;
                (hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hash.to_string())
            })
    });
    let hash = cached.unwrap_or_else(|| KNOWN.to_string());
    format!("https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/{APP}/{hash}.jpg")
}

pub fn steam_id64(account: u32) -> u64 {
    STEAM_ID_BASE + account as u64
}

pub fn modified_unix(path: &Path) -> Option<i64> {
    let t = fs::metadata(path).ok()?.modified().ok()?;
    Some(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
}

/// The running game: its pid and when it started (unix seconds).
#[cfg(windows)]
pub fn game_process() -> Option<(u32, i64)> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = None;
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let end = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if name.eq_ignore_ascii_case(GAME_EXE) {
                    found = Some(entry.th32ProcessID);
                    break;
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        let pid = found?;
        Some((pid, process_start(pid).unwrap_or_else(|| chrono::Local::now().timestamp())))
    }
}

#[cfg(windows)]
unsafe fn process_start(pid: u32) -> Option<i64> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let (mut created, mut exited, mut kernel, mut user) =
            (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        let ok = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user).is_ok();
        let _ = CloseHandle(handle);
        if !ok {
            return None;
        }
        // 100 ns ticks since 1601.
        let ticks = ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64;
        Some((ticks / 10_000_000) as i64 - 11_644_473_600)
    }
}

#[cfg(not(windows))]
pub fn game_process() -> Option<(u32, i64)> {
    None
}
