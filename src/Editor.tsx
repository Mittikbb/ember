import { useRef } from "react";
import { PHASES, VARIABLES, type ImageKind, type Lang, type ShownPhase, type StateCfg, type TimerKind } from "./api";
import { useT, type Key } from "./i18n";

const IMAGES: ImageKind[] = ["heroIcon", "heroCard", "rank", "deadlock", "app"];
const SMALL: ImageKind[] = ["rank", "heroIcon", "deadlock", "none"];
const TIMERS: TimerKind[] = ["state", "match", "queue", "none"];

function Chips<T extends string>({ items, value, label, onChange }: { items: T[]; value: T; label: (v: T) => string; onChange: (v: T) => void }) {
  return (
    <div className="chips">
      {items.map((it) => (
        <button key={it} className={`chip ${it === value ? "on" : ""}`} onClick={() => onChange(it)}>
          {label(it)}
        </button>
      ))}
    </div>
  );
}

type Field = "details" | "state";

export function PhaseChips({ phase, onPick, live, states }: { phase: ShownPhase; onPick: (p: ShownPhase) => void; live: string; states: Record<ShownPhase, StateCfg> }) {
  const t = useT();
  return (
    <div className="chips">
      {PHASES.map((p) => (
        <button
          key={p}
          className={`chip ${p === phase ? "on" : ""} ${states[p]?.enabled === false ? "off" : ""}`}
          onClick={() => onPick(p)}
          title={t(`hint.${p}` as Key)}
        >
          {p === live && <span className="live-dot" aria-label={t("live.now")} />}
          {t(`phase.${p}` as Key)}
        </button>
      ))}
    </div>
  );
}

export function Editor({
  cfg,
  onChange,
  onReset,
}: {
  cfg: StateCfg;
  onChange: (cfg: StateCfg) => void;
  onReset: (lang: Lang) => void;
}) {
  const t = useT();
  const refs = { details: useRef<HTMLInputElement>(null), state: useRef<HTMLInputElement>(null) };
  const focused = useRef<Field>("details");

  const insert = (name: string) => {
    const field = focused.current;
    const el = refs[field].current;
    const value = cfg[field];
    const token = `{${name}}`;
    const at = el?.selectionStart ?? value.length;
    const end = el?.selectionEnd ?? at;
    onChange({ ...cfg, [field]: value.slice(0, at) + token + value.slice(end) });
    requestAnimationFrame(() => {
      el?.focus();
      el?.setSelectionRange(at + token.length, at + token.length);
    });
  };

  const line = (field: Field, label: string, placeholder: string) => (
    <label className="field">
      <span className="label">{label}</span>
      <input
        ref={refs[field]}
        value={cfg[field]}
        placeholder={placeholder}
        spellCheck={false}
        onFocus={() => (focused.current = field)}
        onChange={(e) => onChange({ ...cfg, [field]: e.target.value })}
      />
    </label>
  );

  return (
    <div className={`editor ${cfg.enabled ? "" : "disabled"}`}>
      <div className="row between">
        <label className="switch">
          <input type="checkbox" checked={cfg.enabled} onChange={(e) => onChange({ ...cfg, enabled: e.target.checked })} />
          <span className="track" />
          <span>{cfg.enabled ? t("editor.on") : t("editor.off")}</span>
        </label>
        <div className="reset">
          {t("editor.reset")}
          <button className="link" onClick={() => onReset("en")}>
            EN
          </button>
          <button className="link" onClick={() => onReset("ru")}>
            RU
          </button>
        </div>
      </div>

      <div className="fields">
        {line("details", t("editor.line1"), t("editor.line1Placeholder"))}
        {line("state", t("editor.line2"), "{rank}")}
      </div>

      <div className="vars">
        {VARIABLES.map((v) => (
          <button
            key={v.name}
            className={`var ${v.after || v.memoryOnly ? "after" : ""} ${v.live ? "live" : ""}`}
            onClick={() => insert(v.name)}
            title={
              v.memoryOnly ? t("var.memoryTitle") : v.live ? t("var.liveTitle") : v.after ? t("var.afterTitle") : undefined
            }
          >
            <code>{`{${v.name}}`}</code> {t(`var.${v.name}` as Key)}
          </button>
        ))}
      </div>
      <p className="hint">
        <code>||</code> {t("editor.hint")}
      </p>

      <h3>{t("editor.picture")}</h3>
      <Chips items={IMAGES} value={cfg.image} label={(v) => t(`image.${v}` as Key)} onChange={(image) => onChange({ ...cfg, image })} />
      <h3>{t("editor.small")}</h3>
      <Chips items={SMALL} value={cfg.small} label={(v) => t(`image.${v}` as Key)} onChange={(small) => onChange({ ...cfg, small })} />
      <h3>{t("editor.timer")}</h3>
      <Chips items={TIMERS} value={cfg.timer} label={(v) => t(`timer.${v}` as Key)} onChange={(timer) => onChange({ ...cfg, timer })} />
    </div>
  );
}
