import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { toAppError } from "../api/tauri";
import type { ModCatalogEntry, ModsState, Settings, SystemInfo } from "../api/types";
import { formatRam } from "../lib/format";
import { Close } from "./Icons";

type Tab = "general" | "mods" | "performance" | "advanced";
const TABS: Array<[Tab, string]> = [
  ["general", "General"],
  ["mods", "Mods"],
  ["performance", "Performance"],
  ["advanced", "Advanced"],
];

function Switch({
  checked,
  label,
  disabled,
  onChange,
}: {
  checked: boolean;
  label: string;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      className="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    />
  );
}

interface Props {
  settings: Settings;
  system: SystemInfo;
  catalog: ModCatalogEntry[];
  modsState: ModsState | null;
  busy: boolean;
  canVerify: boolean;
  onSave: (next: Settings) => Promise<void>;
  onVerify: () => void;
  onClose: () => void;
}

async function browse(directory: boolean): Promise<string | null> {
  const picked = await open({ directory, multiple: false });
  return typeof picked === "string" ? picked : null;
}

export function SettingsPanel({ settings, system, catalog, modsState, busy, canVerify, onSave, onVerify, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const [draft, setDraft] = useState<Settings>(settings);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  const patch = (changes: Partial<Settings>) => {
    setDraft((d) => ({ ...d, ...changes }));
    setSaved(false);
  };

  const toggleMod = (slug: string, on: boolean) =>
    patch({
      disabledMods: on ? draft.disabledMods.filter((s) => s !== slug) : [...draft.disabledMods, slug],
    });

  const autoRam = draft.maxRamMb === null;
  const ramValue = draft.maxRamMb ?? system.recommendedRamMb;
  const ramMax = Math.max(1024, Math.floor(system.totalRamMb / 256) * 256);

  const save = async () => {
    setError(null);
    try {
      await onSave(draft);
      setSaved(true);
    } catch (e) {
      setError(toAppError(e).message);
    }
  };

  const pick = async (directory: boolean, key: "javaPath" | "gameDir") => {
    const path = await browse(directory);
    if (path) patch({ [key]: path });
  };

  return (
    <div className="drawer-backdrop" onPointerDown={onClose}>
      <aside
        className="drawer"
        role="dialog"
        aria-label="Settings"
        onPointerDown={(e) => e.stopPropagation()}
      >
        <header className="drawer__header">
          <h2>Settings</h2>
          <button className="icon-btn" aria-label="Close settings" onClick={onClose}>
            <Close />
          </button>
        </header>

        <div className="tabs" role="tablist">
          {TABS.map(([id, label]) => (
            <button
              key={id}
              role="tab"
              aria-selected={tab === id}
              className={`tab ${tab === id ? "is-active" : ""}`}
              onClick={() => setTab(id)}
            >
              {label}
            </button>
          ))}
        </div>

        <div className="drawer__body">
          {tab === "general" && (
            <>
              <label className="field">
                <span className="field__label">Player name</span>
                <input
                  value={draft.username}
                  maxLength={16}
                  spellCheck={false}
                  onChange={(e) => patch({ username: e.target.value })}
                />
                <span className="field__hint">Offline profile stored on this computer. Not a Microsoft account.</span>
              </label>

              <div className="field">
                <span className="field__label">Installation directory</span>
                <div className="field__input-row">
                  <input
                    value={draft.gameDir ?? ""}
                    placeholder={system.defaultGameDir}
                    spellCheck={false}
                    onChange={(e) => patch({ gameDir: e.target.value })}
                  />
                  <button className="btn btn--ghost" onClick={() => void pick(true, "gameDir")}>
                    Browse
                  </button>
                </div>
              </div>
            </>
          )}

          {tab === "mods" && (
            <>
              <div className="mod-row">
                <span>
                  <span className="mod-row__name">Fabric &amp; performance mods</span>
                  <span className="mod-row__desc">
                    Installed automatically per version. Versions without Fabric support start as vanilla.
                  </span>
                </span>
                <Switch label="Enable mods" checked={draft.modsEnabled} onChange={(v) => patch({ modsEnabled: v })} />
              </div>
              {modsState?.error && (
                <p className="mod-row__desc">
                  Mods could not be downloaded ({modsState.error}). This version runs as vanilla. Use "Verify selected version" in
                  Advanced to retry.
                </p>
              )}
              {[...new Set(catalog.map((m) => m.category))].map((category) => (
                <div className="mod-group" key={category}>
                  <h3 className="mod-group__title">{category}</h3>
                  {catalog
                    .filter((m) => m.category === category)
                    .map((m) => {
                      const on = !draft.disabledMods.includes(m.slug);
                      const skipped = modsState?.skipped.includes(m.slug);
                      return (
                        <div className={`mod-row ${on && draft.modsEnabled ? "" : "is-off"}`} key={m.slug}>
                          <span>
                            <span className="mod-row__name">
                              {m.name}
                              {skipped && <span className="mod-row__tag">unavailable for this version</span>}
                            </span>
                            <span className="mod-row__desc">{m.description}</span>
                          </span>
                          <Switch
                            label={m.name}
                            checked={on}
                            disabled={!draft.modsEnabled}
                            onChange={(v) => toggleMod(m.slug, v)}
                          />
                        </div>
                      );
                    })}
                </div>
              ))}
              <div className="mods-actions">
                <button className="btn btn--ghost" onClick={() => patch({ disabledMods: [] })}>
                  Enable all
                </button>
                <button
                  className="btn btn--ghost"
                  onClick={() => patch({ disabledMods: catalog.map((m) => m.slug) })}
                >
                  Disable all
                </button>
              </div>
              <span className="field__hint">Changes apply the next time you press Play.</span>
            </>
          )}

          {tab === "performance" && (
            <>
              <div className="field">
                <div className="field__row">
                  <span className="field__label">Memory</span>
                  <span className="field__value">
                    {formatRam(ramValue)}
                    {autoRam && <em> · automatic</em>}
                  </span>
                </div>
                <input
                  type="range"
                  min={1024}
                  max={ramMax}
                  step={256}
                  value={Math.min(ramValue, ramMax)}
                  onChange={(e) => patch({ maxRamMb: Number(e.target.value) })}
                />
                <div className="field__row">
                  <span className="field__hint">Of {formatRam(system.totalRamMb)} installed</span>
                  {!autoRam && (
                    <button className="link" onClick={() => patch({ maxRamMb: null })}>
                      Use automatic
                    </button>
                  )}
                </div>
              </div>

              <div className="field">
                <span className="field__label">Java executable</span>
                <div className="field__input-row">
                  <input
                    value={draft.javaPath ?? ""}
                    placeholder="Automatic (managed by launcher)"
                    spellCheck={false}
                    onChange={(e) => patch({ javaPath: e.target.value })}
                  />
                  <button className="btn btn--ghost" onClick={() => void pick(false, "javaPath")}>
                    Browse
                  </button>
                </div>
              </div>
            </>
          )}

          {tab === "advanced" && (
            <>
              <label className="field">
                <span className="field__label">Advanced launch arguments</span>
                <textarea
                  rows={3}
                  value={draft.jvmArgs}
                  placeholder="-XX:+UseG1GC -Dexample=value"
                  spellCheck={false}
                  onChange={(e) => patch({ jvmArgs: e.target.value })}
                />
                <span className="field__hint">Extra JVM arguments, added before the game starts.</span>
              </label>

              <div className="field">
                <span className="field__label">Maintenance</span>
                <button className="btn btn--ghost" disabled={!canVerify || busy} onClick={onVerify}>
                  Verify selected version
                </button>
                <span className="field__hint">Re-checks every file against Mojang's checksums and repairs damage.</span>
              </div>

              <div className="field">
                <span className="field__label">System</span>
                <span className="field__hint">
                  {system.os} · {system.arch} · {formatRam(system.totalRamMb)} RAM
                </span>
              </div>
            </>
          )}
        </div>

        <footer className="drawer__footer">
          <span className={`drawer__message ${error ? "is-error" : ""}`}>{error ?? (saved ? "Saved" : "")}</span>
          <button className="btn btn--primary" onClick={() => void save()}>
            Save
          </button>
        </footer>
      </aside>
    </div>
  );
}
