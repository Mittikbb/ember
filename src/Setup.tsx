import { useEffect, useState } from "react";
import { api, type ButtonCfg, type Settings, type Status } from "./api";
import { message, rich, useT, type Key } from "./i18n";

const PORTAL = "https://discord.com/developers/applications";
const STEAM_PROFILE = "https://steamcommunity.com/profiles/{steam_id}";

const MEMORY_TONE: Record<string, string> = {
  waiting: "muted",
  loading: "muted",
  attached: "ok",
  stale: "warn",
  error: "warn",
};

function MemoryLine({ status }: { status: Status | null }) {
  const t = useT();
  const s = status?.memory;
  if (!s || !(s.state in MEMORY_TONE)) return <p className="muted">…</p>;
  const detail = message(t, s.message);
  return (
    <p className={MEMORY_TONE[s.state]}>
      {t(`mem.${s.state}` as Key)}
      {detail ? `: ${detail}` : ""}
    </p>
  );
}

function RiskDialog({ onCancel, onAccept }: { onCancel: () => void; onAccept: () => void }) {
  const t = useT();
  const [sure, setSure] = useState(false);
  return (
    <div className="modal-back" onClick={onCancel}>
      <div className="modal" role="dialog" aria-modal onClick={(e) => e.stopPropagation()}>
        <h2>{t("risk.title")}</h2>
        <ul>
          {(["risk.1", "risk.2", "risk.3", "risk.4", "risk.5"] as const).map((k) => (
            <li key={k}>{rich(t(k))}</li>
          ))}
        </ul>
        <label className="check">
          <input type="checkbox" checked={sure} onChange={(e) => setSure(e.target.checked)} />
          <span>{t("risk.check")}</span>
        </label>
        <div className="modal-actions">
          <button className="btn" onClick={onCancel}>
            {t("risk.cancel")}
          </button>
          <button className="btn danger" disabled={!sure} onClick={onAccept}>
            {t("risk.accept")}
          </button>
        </div>
      </div>
    </div>
  );
}

export function Setup({ settings, status, onChange }: { settings: Settings; status: Status | null; onChange: (s: Settings) => void }) {
  const t = useT();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [copied, setCopied] = useState(false);
  const [asking, setAsking] = useState(false);
  useEffect(() => {
    api.getAutostart().then(setAutostart).catch(() => setAutostart(null));
  }, []);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) => onChange({ ...settings, [key]: value });
  const setButton = (i: number, b: ButtonCfg | null) => {
    const list = settings.buttons.slice();
    if (b) list[i] = b;
    else list.splice(i, 1);
    set("buttons", list);
  };
  const idValid = /^\d{17,20}$/.test(settings.clientId.trim());
  const presets: { name: string; button: ButtonCfg }[] = [
    { name: t("s3.steam"), button: { label: t("s3.steam"), url: STEAM_PROFILE } },
    { name: t("s3.custom"), button: { label: "", url: "https://" } },
  ];

  return (
    <div className="setup">
      <section>
        <h2>
          <span className="step">1</span> {t("s1.title")}
        </h2>
        <p>{t("s1.intro")}</p>
        <ol>
          <li>
            {t("s1.open")}{" "}
            <button className="link" onClick={() => api.openUrl(PORTAL)}>
              Discord Developer Portal
            </button>{" "}
            {rich(t("s1.openAfter"))}
          </li>
          <li>{rich(t("s1.name"))}</li>
          <li>{rich(t("s1.copy"))}</li>
        </ol>
        <label className="field">
          <span className="label">Application ID</span>
          <input
            value={settings.clientId}
            placeholder="1234567890123456789"
            spellCheck={false}
            onChange={(e) => set("clientId", e.target.value.replace(/[^\d]/g, ""))}
          />
        </label>
        {settings.clientId && !idValid && <p className="warn">{t("s1.badId")}</p>}
        {status?.lumenClash && <p className="warn">{t("s1.lumenClash")}</p>}

        <h3>{t("icon.title")}</h3>
        <p>{t("icon.intro")}</p>
        <ol>
          <li>
            <button className="link" onClick={() => api.saveIcon()}>
              {t("icon.save")}
            </button>{" "}
            {rich(t("icon.saveAfter"))}
          </li>
          <li>{rich(t("icon.upload"))}</li>
          <li>{t("icon.turnOn")}</li>
        </ol>
        <label className="switch">
          <input type="checkbox" checked={settings.deadlockAsset} onChange={(e) => set("deadlockAsset", e.target.checked)} />
          <span className="track" />
          <span>{t("icon.switch")}</span>
        </label>
        <p className="note">{t("icon.note")}</p>
      </section>

      <section>
        <h2>
          <span className="step">2</span> Deadlock
        </h2>
        {status?.deadlockFound ? (
          <p className="ok">{settings.deadlockDir ? t("s2.foundCustom") : t("s2.found")}</p>
        ) : (
          <p className="warn">{t("s2.notFound")}</p>
        )}
        <label className="field">
          <span className="label">{t("s2.folder")}</span>
          <input
            value={settings.deadlockDir}
            placeholder="D:\SteamLibrary\steamapps\common\Deadlock"
            spellCheck={false}
            onChange={(e) => set("deadlockDir", e.target.value)}
          />
        </label>
        <p>
          {t("s2.condebug")}{" "}
          <code className="copy" onClick={() => navigator.clipboard.writeText("-condebug").then(() => setCopied(true))}>
            -condebug
          </code>
          {copied && <span className="ok"> {t("s2.copied")}</span>}. {t("s2.safe")}
        </p>
        {status?.logStale && <p className="warn">{t("s2.stale")}</p>}
      </section>

      <section>
        <h2>
          <span className="step">3</span> {t("s3.title")}
        </h2>
        <p>{t("s3.intro")}</p>
        {settings.buttons.map((b, i) => (
          <div className="button-row" key={i}>
            <input value={b.label} placeholder={t("s3.label")} maxLength={32} onChange={(e) => setButton(i, { ...b, label: e.target.value })} />
            <input value={b.url} placeholder="https://…" spellCheck={false} onChange={(e) => setButton(i, { ...b, url: e.target.value })} />
            <button className="link" onClick={() => setButton(i, null)}>
              {t("s3.remove")}
            </button>
          </div>
        ))}
        {settings.buttons.length < 2 && (
          <div className="chips">
            {presets.map((p) => (
              <button key={p.name} className="chip" onClick={() => set("buttons", [...settings.buttons, p.button])}>
                + {p.name}
              </button>
            ))}
          </div>
        )}
      </section>

      <section>
        <h2>
          <span className="step">4</span> {t("s4.title")}
        </h2>
        <label className="field inline">
          <span className="label">{t("s4.postGame")}</span>
          <input
            type="number"
            min={0}
            max={30}
            value={settings.postGameMinutes}
            onChange={(e) => set("postGameMinutes", Math.max(0, Math.min(30, Number(e.target.value) || 0)))}
          />
        </label>
        <label className="field inline">
          <span className="label">{t("s4.lang")}</span>
          <select value={settings.lang} onChange={(e) => set("lang", e.target.value as Settings["lang"])}>
            <option value="en">English</option>
            <option value="ru">Русский</option>
          </select>
        </label>
        {autostart !== null && (
          <label className="switch">
            <input
              type="checkbox"
              checked={autostart}
              onChange={(e) => {
                const on = e.target.checked;
                api.setAutostart(on).then(() => setAutostart(on));
              }}
            />
            <span className="track" />
            <span>{t("s4.autostart")}</span>
          </label>
        )}
      </section>

      <section className="risky">
        <h2>
          <span className="step warn-step">!</span> {t("mem.title")}
        </h2>
        <p>{t("mem.intro")}</p>
        <label className="switch">
          <input
            type="checkbox"
            checked={settings.memory}
            onChange={(e) => (e.target.checked ? setAsking(true) : set("memory", false))}
          />
          <span className="track" />
          <span>{t("mem.switch")}</span>
        </label>
        {settings.memory && <MemoryLine status={status} />}
      </section>

      {asking && (
        <RiskDialog
          onCancel={() => setAsking(false)}
          onAccept={() => {
            setAsking(false);
            set("memory", true);
          }}
        />
      )}
    </div>
  );
}
