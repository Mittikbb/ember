import { useEffect, useState } from "react";
import type { Activity } from "./api";
import { useT } from "./i18n";
import logo from "../icon.svg";

/** Elapsed time the way Discord shows it: 4:07, 1:02:33. */
function elapsed(startMs: number, nowMs: number) {
  const s = Math.max(0, Math.floor((nowMs - startMs) / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}

/** A link as is; an asset key of the user's Discord app as its local copy. */
function asset(image: string) {
  return image.includes("://") ? image : `/${image}-icon.png`;
}

function useNow() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  return now;
}

/** A copy of the activity card in a Discord profile. */
export function DiscordCard({ activity, appName = "Deadlock", empty }: { activity: Activity | null; appName?: string; empty: string }) {
  const t = useT();
  const now = useNow();
  if (!activity) {
    return (
      <div className="dc-card dc-empty">
        <span>{empty}</span>
      </div>
    );
  }
  const a = activity.assets ?? {};
  const large = a.large_image ? asset(a.large_image) : logo;
  const small = a.small_image && asset(a.small_image);
  return (
    <div className="dc-card">
      <div className="dc-head">
        <span>{t("card.playing")}</span>
        <span className="dc-dots">···</span>
      </div>
      <div className="dc-body">
        <div className="dc-art" title={a.large_text}>
          <img className="dc-large" src={large} alt="" />
          {small && <img className="dc-small" src={small} alt="" title={a.small_text} />}
        </div>
        <div className="dc-text">
          <div className="dc-name">{appName}</div>
          {activity.details && <div className="dc-line">{activity.details}</div>}
          {activity.state && <div className="dc-line">{activity.state}</div>}
          {activity.timestamps && (
            <div className="dc-time">
              <svg viewBox="0 0 24 24" width="13" height="13" aria-hidden>
                <path
                  fill="currentColor"
                  d="M20.97 4.06c0 .18.08.35.24.43.55.28.9.82 1.04 1.42.3 1.24.75 3.7.75 7.09v4.91a3.09 3.09 0 0 1-5.85 1.38l-1.76-3.51a1.09 1.09 0 0 0-1.23-.55c-.57.13-1.36.27-2.16.27s-1.6-.14-2.16-.27c-.49-.11-1 .1-1.23.55l-1.76 3.51A3.09 3.09 0 0 1 1 17.91V13c0-3.38.46-5.85.75-7.1.15-.6.49-1.13 1.04-1.4a.47.47 0 0 0 .24-.44c0-.7.48-1.32 1.2-1.47l2.93-.62c.5-.1 1 .06 1.36.4.35.34.78.71 1.28.68a42.4 42.4 0 0 1 4.4 0c.5.03.93-.34 1.28-.69.35-.33.86-.5 1.36-.39l2.94.62c.7.15 1.19.78 1.19 1.47ZM20 11.5a1.5 1.5 0 1 0-3 0 1.5 1.5 0 0 0 3 0ZM15.5 12a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3ZM5 13a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z"
                />
              </svg>
              {elapsed(activity.timestamps.start, now)}
            </div>
          )}
        </div>
      </div>
      {activity.buttons && activity.buttons.length > 0 && (
        <div className="dc-buttons">
          {activity.buttons.map((b, i) => (
            <div key={i} className="dc-button" title={b.url}>
              {b.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
