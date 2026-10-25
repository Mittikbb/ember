import { invoke } from "@tauri-apps/api/core";

export type Phase =
  | "closed"
  | "launching"
  | "hideout"
  | "queue"
  | "found"
  | "loading"
  | "match"
  | "postGame"
  | "sandbox"
  | "spectating";

export type ShownPhase = Exclude<Phase, "closed">;
export type ImageKind = "app" | "deadlock" | "heroIcon" | "heroCard" | "rank" | "none";
export type TimerKind = "none" | "state" | "match" | "queue";
export type Lang = "ru" | "en";

export interface StateCfg {
  enabled: boolean;
  details: string;
  state: string;
  image: ImageKind;
  small: ImageKind;
  timer: TimerKind;
}

export interface ButtonCfg {
  label: string;
  url: string;
}

export interface Settings {
  enabled: boolean;
  clientId: string;
  deadlockAsset: boolean;
  deadlockDir: string;
  postGameMinutes: number;
  lang: Lang;
  memory: boolean;
  states: Record<ShownPhase, StateCfg>;
  buttons: ButtonCfg[];
}

export interface Activity {
  type: number;
  details?: string;
  state?: string;
  timestamps?: { start: number };
  assets?: { large_image?: string; large_text?: string; small_image?: string; small_text?: string };
  buttons?: ButtonCfg[];
}

export interface Facts {
  phase: Phase;
  since: number;
  matchStart: number | null;
  queueStart: number | null;
  hero: { name: string; line: string | null; icon: string | null; card: string | null } | null;
  rank: { name: string; image: string | null } | null;
  stats: { kills: number; deaths: number; assists: number; souls: number; won: boolean } | null;
  matchId: number | null;
  accountId: number | null;
}

export interface Status {
  running: boolean;
  deadlockFound: boolean;
  logStale: boolean;
  facts: Facts;
  discord: { state: "off" | "idle" | "connecting" | "ready" | "error" | ""; message: string | null; user: string | null };
  activity: Activity | null;
  lumenClash: boolean;
  heroesLoaded: boolean;
  memory: { state: "off" | "waiting" | "attached" | "loading" | "stale" | "error" | ""; message: string | null };
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  getStatus: () => invoke<Status>("get_status"),
  preview: (settings: Settings, phase: ShownPhase) => invoke<Activity | null>("preview", { settings, phase }),
  preset: (lang: Lang) => invoke<Record<ShownPhase, StateCfg>>("preset", { lang }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  saveIcon: () => invoke<void>("save_icon"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (on: boolean) => invoke<void>("set_autostart", { on }),
};

/** The phases a status can be set up for, in the order the editor shows them. */
export const PHASES: ShownPhase[] = [
  "launching",
  "hideout",
  "queue",
  "found",
  "loading",
  "match",
  "postGame",
  "sandbox",
  "spectating",
];

/** `after`: known after the match; `live`: also during it, with memory reading on. */
export const VARIABLES: { name: string; after?: boolean; live?: boolean; memoryOnly?: boolean }[] = [
  { name: "hero" },
  { name: "rank" },
  { name: "hero_line" },
  { name: "result", after: true },
  { name: "kda", after: true, live: true },
  { name: "souls", after: true, live: true },
  { name: "kills", after: true, live: true },
  { name: "deaths", after: true, live: true },
  { name: "assists", after: true, live: true },
  { name: "last_hits", after: true, live: true },
  { name: "level", after: true, live: true },
  { name: "duration", after: true },
  { name: "mode", after: true, live: true },
  { name: "round", live: true, memoryOnly: true },
  { name: "score", live: true, memoryOnly: true },
  { name: "match_id" },
  { name: "steam_id" },
  { name: "account_id" },
];
