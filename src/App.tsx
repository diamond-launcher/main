import { useEffect, useState, type CSSProperties } from "react";
import { ErrorNotice } from "./components/ErrorNotice";
import { Diamond, Gear } from "./components/Icons";
import { InstallStatus } from "./components/InstallStatus";
import { PlayButton, type PlayMode } from "./components/PlayButton";
import { SettingsPanel } from "./components/SettingsPanel";
import { TitleBar } from "./components/TitleBar";
import { VersionPicker } from "./components/VersionPicker";
import { useLauncher } from "./hooks/useLauncher";
import { relativeTime } from "./lib/format";

export default function App() {
  const launcher = useLauncher();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { selected, settings, system, install, game, error } = launcher;

  const mode: PlayMode = launcher.isInstalling
    ? "busy"
    : launcher.isGameActive
      ? "running"
      : !selected
        ? "disabled"
        : selected.installed
          ? "play"
          : "install";

  const onPrimary = () => void (mode === "install" ? launcher.installSelected() : launcher.play());

  const statusLine = (() => {
    if (game?.state === "starting") return "Starting Minecraft…";
    if (game?.state === "running") return "Minecraft is running";
    if (!selected) return launcher.versions ? "" : "Loading versions…";
    if (!selected.installed) return "Not installed";
    const modCount = launcher.modsState?.mods.length ?? 0;
    const modded = launcher.modsState?.profileId && modCount > 0;
    return modded ? `Ready · Fabric · ${modCount} mods` : "Ready · Vanilla";
  })();

  const lastPlayed = settings?.lastPlayed;
  const [intro, setIntro] = useState(true);
  useEffect(() => {
    const timer = window.setTimeout(() => setIntro(false), 1900);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <div className="app">
      {intro && (
        <div className="splash" aria-hidden="true">
          <Diamond className="splash__gem" />
          <div className="splash__name">Diamond Launcher</div>
          <div className="splash__bar" />
        </div>
      )}
      <TitleBar />

      <main className="stage">
        <div className="wordmark enter" style={{ "--i": 0 } as CSSProperties}>
          Minecraft
        </div>

        <section className="hero enter" style={{ "--i": 1 } as CSSProperties}>
          <div className="hero__version" key={selected?.id ?? "none"}>
            {selected?.id ?? "—"}
          </div>
          <div className="hero__kind">
            {selected ? (selected.kind === "release" ? "Release" : "Snapshot") : "\u00A0"}
            {launcher.versions?.offline && " · Offline list"}
          </div>
        </section>

        <div className="enter" style={{ "--i": 2 } as CSSProperties}>
          <VersionPicker
            versions={launcher.versions}
            selected={selected}
            showSnapshots={launcher.showSnapshots}
            disabled={launcher.isInstalling || launcher.isGameActive}
            onSelect={launcher.select}
            onShowSnapshots={launcher.setShowSnapshots}
          />
        </div>

        <div className="enter" style={{ "--i": 3 } as CSSProperties}>
          <PlayButton mode={mode} onClick={onPrimary} />
        </div>

        <div className="status enter" style={{ "--i": 4 } as CSSProperties} aria-live="polite">
          {install ? (
            <InstallStatus install={install} />
          ) : error ? (
            <ErrorNotice
              error={error}
              logTail={game?.state === "crashed" ? game.logTail : []}
              onDismiss={launcher.dismissError}
              onRetry={launcher.versions ? undefined : () => void launcher.reloadVersions()}
            />
          ) : (
            <span className="status__text">
              {statusLine}
              {launcher.isGameActive && (
                <button className="link" onClick={() => void launcher.stop()}>
                  Stop
                </button>
              )}
            </span>
          )}
        </div>
      </main>

      <footer className="footer enter" style={{ "--i": 5 } as CSSProperties}>
        <span className="footer__last">
          {lastPlayed ? (
            <>
              Last played <strong>{lastPlayed.version}</strong> · {relativeTime(lastPlayed.at)}
            </>
          ) : (
            "Not played yet"
          )}
        </span>
        <button className="icon-btn" aria-label="Settings" onClick={() => setSettingsOpen(true)} disabled={!settings}>
          <Gear width={18} height={18} />
        </button>
      </footer>

      {settingsOpen && settings && system && (
        <SettingsPanel
          settings={settings}
          system={system}
          catalog={launcher.modCatalog}
          modsState={launcher.modsState}
          busy={launcher.isInstalling || launcher.isGameActive}
          canVerify={Boolean(selected)}
          onSave={launcher.saveSettings}
          onVerify={() => {
            setSettingsOpen(false);
            void launcher.installSelected(true);
          }}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </div>
  );
}
