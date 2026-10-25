import { useEffect, useRef, useState } from "react";
import { api, type Activity, type Settings, type ShownPhase, type Status } from "./api";
import { DiscordCard } from "./DiscordCard";
import { Editor, PhaseChips } from "./Editor";
import { LangContext, message, useT, type Key, type T } from "./i18n";
import { Setup } from "./Setup";
import logo from "../icon.svg";

type Tab = "states" | "setup";

function discordLine(t: T, s: Status | null, settings: Settings) {
  if (!settings.enabled) return { tone: "muted", text: t("discord.off") };
  const d = s?.discord;
  const text = message(t, d?.message);
  switch (d?.state) {
    case "ready":
      return { tone: "ok", text: d.user ? `Discord: ${d.user}` : t("discord.connected") };
    case "connecting":
      return { tone: "muted", text: t("discord.connecting") };
    case "error":
      return { tone: "bad", text: text ?? t("discord.error") };
    case "idle":
    case "off":
      return { tone: d.message === "discord.noClientId" ? "bad" : "muted", text: text ?? t("discord.waiting") };
    default:
      return { tone: "muted", text: "…" };
  }
}

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  return (
    <LangContext.Provider value={settings?.lang ?? "en"}>
      <Window settings={settings} setSettings={setSettings} />
    </LangContext.Provider>
  );
}

function Window({ settings, setSettings }: { settings: Settings | null; setSettings: (s: Settings) => void }) {
  const t = useT();
  const [status, setStatus] = useState<Status | null>(null);
  const [tab, setTab] = useState<Tab>("states");
  const [phase, setPhase] = useState<ShownPhase>("hideout");
  const [preview, setPreview] = useState<Activity | null>(null);
  const [saved, setSaved] = useState<"idle" | "saving" | "saved" | string>("idle");
  const firstStatus = useRef(true);
  const saveTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    api.getSettings().then((s) => {
      setSettings(s);
      if (!s.clientId) setTab("setup");
    });
    const poll = () =>
      api.getStatus().then((st) => {
        setStatus(st);
        if (firstStatus.current) {
          firstStatus.current = false;
          if (st.facts.phase !== "closed") setPhase(st.facts.phase);
        }
      });
    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, []);

  // The preview follows every edit, and the live data once a second.
  const livePhase = status?.facts.phase;
  const heroesLoaded = status?.heroesLoaded;
  useEffect(() => {
    if (!settings) return;
    let stale = false;
    api.preview(settings, phase).then((a) => !stale && setPreview(a));
    return () => {
      stale = true;
    };
  }, [settings, phase, livePhase, heroesLoaded, status?.facts.hero?.name, status?.facts.stats]);

  if (!settings) return <div className="loading" />;

  const update = (next: Settings) => {
    setSettings(next);
    setSaved("saving");
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      api
        .saveSettings(next)
        .then(() => setSaved("saved"))
        .catch((e) => setSaved(String(e)));
    }, 400);
  };

  const cfg = settings.states[phase];
  const d = discordLine(t, status, settings);
  const now = status?.running ? `Deadlock · ${t(`now.${status.facts.phase}` as Key)}` : t("game.closed");

  return (
    <div className="app">
      <header>
        <div className="brand">
          <img src={logo} alt="" />
          <span>Ember</span>
        </div>
        <nav>
          <button className={tab === "states" ? "on" : ""} onClick={() => setTab("states")}>
            {t("tab.states")}
          </button>
          <button className={tab === "setup" ? "on" : ""} onClick={() => setTab("setup")}>
            {t("tab.setup")}
          </button>
        </nav>
        <div className="pills">
          <span className={`pill ${status?.running ? "ok" : "muted"}`}>
            <i />
            {now}
            {status?.facts.hero && status.running ? ` · ${status.facts.hero.name}` : ""}
          </span>
          <span className={`pill ${d.tone}`} title={d.text}>
            <i />
            {d.text}
          </span>
          <div className="lang" title={t("lang.title")}>
            {(["en", "ru"] as const).map((l) => (
              <button key={l} className={settings.lang === l ? "on" : ""} onClick={() => update({ ...settings, lang: l })}>
                {l.toUpperCase()}
              </button>
            ))}
          </div>
          <label className="switch compact" title={t("toggle.title")}>
            <input type="checkbox" checked={settings.enabled} onChange={(e) => update({ ...settings, enabled: e.target.checked })} />
            <span className="track" />
          </label>
        </div>
      </header>

      {tab === "setup" ? (
        <main className="scroll">
          <Setup settings={settings} status={status} onChange={update} />
        </main>
      ) : (
        <main className="split">
          <aside>
            <div className="eyebrow">{t("states.friends", { phase: t(`phase.${phase}` as Key) })}</div>
            <DiscordCard activity={preview} empty={t("states.emptyDisabled")} />
            <p className="note">{livePhase === phase ? t("states.noteLive") : t("states.noteSample")}</p>

            <div className="eyebrow">{t("states.inDiscord")}</div>
            <DiscordCard activity={status?.activity ?? null} empty={status?.running ? t("states.notSent") : t("states.gameClosed")} />
            <div className={`saved ${saved === "saved" || saved === "saving" || saved === "idle" ? "" : "bad"}`}>
              {saved === "saving" ? t("states.saving") : saved === "saved" ? t("states.saved") : saved === "idle" ? "" : saved}
            </div>
          </aside>
          <section className="scroll">
            <div className="eyebrow">{t("states.eyebrow")}</div>
            <h1>
              {t("states.title")} <em>{t("states.titleEm")}</em>
            </h1>
            <PhaseChips phase={phase} onPick={setPhase} live={livePhase ?? "closed"} states={settings.states} />
            <Editor
              cfg={cfg}
              onChange={(c) => update({ ...settings, states: { ...settings.states, [phase]: c } })}
              onReset={(lang) =>
                api.preset(lang).then((p) => update({ ...settings, states: { ...settings.states, [phase]: p[phase] } }))
              }
            />
          </section>
        </main>
      )}
    </div>
  );
}
