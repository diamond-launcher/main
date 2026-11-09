import { useEffect, useState } from "react";
import { api } from "../api/tauri";
import type { AppInfo } from "../api/types";
import { Close, Diamond } from "./Icons";

interface Props {
  onClose: () => void;
}

const FALLBACK: AppInfo = {
  version: "—",
  developer: "den3zx",
  website: "https://github.com/den3zx/diamond-launcher",
};

export function InfoPanel({ onClose }: Props) {
  const [info, setInfo] = useState<AppInfo>(FALLBACK);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let disposed = false;
    api
      .appInfo()
      .then((value) => !disposed && setInfo(value))
      .catch(() => undefined);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("keydown", onKey);
    return () => {
      disposed = true;
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  const copyWebsite = async () => {
    try {
      await navigator.clipboard.writeText(info.website);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1400);
    } catch {
      // Clipboard access denied: the address stays selectable.
    }
  };

  return (
    <div className="info-backdrop" onPointerDown={onClose}>
      <section
        className="info-card"
        role="dialog"
        aria-label="About Diamond Launcher"
        onPointerDown={(e) => e.stopPropagation()}
      >
        <header className="info-card__header">
          <button className="icon-btn" aria-label="Close about" onClick={onClose}>
            <Close />
          </button>
        </header>

        <div className="info-card__brand">
          <Diamond className="info-card__gem" />
          <div>
            <h2>Diamond Launcher</h2>
            <span className="info-card__version">v{info.version}</span>
          </div>
        </div>

        <p className="info-card__tagline">
          A minimalist Minecraft launcher with a dark, cyan-accented interface and a fully customizable in-game HUD.
        </p>

        <dl className="info-card__rows">
          <div>
            <dt>Version</dt>
            <dd>{info.version}</dd>
          </div>
          <div>
            <dt>Developer</dt>
            <dd>{info.developer}</dd>
          </div>
          <div>
            <dt>Website</dt>
            <dd>
              <code>{info.website}</code>
              <button className="link" onClick={() => void copyWebsite()}>
                {copied ? "Copied" : "Copy"}
              </button>
            </dd>
          </div>
          <div>
            <dt>Client</dt>
            <dd>Fabric (performance mods + optional HUD)</dd>
          </div>
        </dl>

        <p className="info-card__fine">
          Not affiliated with Mojang Studios or Microsoft. Minecraft is a trademark of Mojang Synergies AB.
        </p>
      </section>
    </div>
  );
}
