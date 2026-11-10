import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { toAppError } from "../api/tauri";
import type { HudModule, HudSettings, ModCatalogEntry, ModsState, Settings, SystemInfo } from "../api/types";
import { formatRam } from "../lib/format";
import { Close } from "./Icons";

type Tab = "general" | "mods" | "performance" | "hud" | "advanced";
const TABS: Array<[Tab, string]> = [
  ["general", "General"],
  ["mods", "Mods"],
  ["performance", "Performance"],
  ["hud", "HUD"],
  ["advanced", "Advanced"],
];

const HUD_MODULES = [
  ["fps", "FPS"],
  ["cps", "CPS"],
  ["keystrokes", "Keys"],
  ["coords", "Coords"],
  ["day", "Day Counter"],
  ["ip", "IP Info"],
  ["armor", "Armor HUD"],
] as const;
type HudKey = (typeof HUD_MODULES)[number][0] | "fullbright" | "time_changer" | "ping" | "toggle_sneak" | "toggle_sprint";

const HUD_DEFAULT_POS: Record<HudKey, { x: number; y: number }> = {
  fps: { x: 0.012, y: 0.012 },
  cps: { x: 0.012, y: 0.06 },
  keystrokes: { x: 0.012, y: 0.112 },
  coords: { x: 0.012, y: 0.355 },
  day: { x: 0.012, y: 0.225 },
  ip: { x: 0.012, y: 0.3 },
  armor: { x: 0.85, y: 0.012 },
  fullbright: { x: 0.85, y: 0.075 },
  time_changer: { x: 0.85, y: 0.135 },
  ping: { x: 0.85, y: 0.2 },
  toggle_sneak: { x: 0.85, y: 0.265 },
  toggle_sprint: { x: 0.85, y: 0.33 },
};

const HUD_DESCRIPTIONS: Record<HudKey, string> = {
  fps: "Frames per second.",
  cps: "Left and right clicks per second.",
  keystrokes: "WASD, mouse buttons, shift and jump.",
  coords: "Player block coordinates.",
  day: "In-game day count.",
  ip: "Connected server address.",
  armor: "Equipped armor with durability.",
  fullbright: "Rendering-level brightness; enable/disable only.",
  time_changer: "Client-side time of day override. Does not affect the server or other players.",
  ping: "Shows your current server ping in milliseconds.",
  toggle_sneak: "Toggle sneak on/off with Shift instead of holding it.",
  toggle_sprint: "Toggle sprint on/off with the sprint key instead of holding it.",
};

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
  onClearModState: (id: string) => Promise<void>;
}

async function browse(directory: boolean): Promise<string | null> {
  const picked = await open({ directory, multiple: false });
  return typeof picked === "string" ? picked : null;
}

export function SettingsPanel({ settings, system, catalog, modsState, busy, canVerify, onSave, onVerify, onClose, onClearModState }: Props) {
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

  const previewRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ key: HudKey; dx: number; dy: number } | null>(null);

  const patchHud = (changes: Partial<HudSettings>) => patch({ hud: { ...draft.hud, ...changes } });

  const patchModule = (key: HudKey, changes: Partial<HudModule>) => {
    const hud: HudSettings = { ...draft.hud, [key]: { ...draft.hud[key], ...changes } };
    patch({ hud });
  };

  const startDrag = (e: ReactPointerEvent<HTMLButtonElement>, key: HudKey) => {
    if (!draft.hud.enabled) return;
    const rect = previewRef.current?.getBoundingClientRect();
    if (!rect) return;
    const module = draft.hud[key];
    dragRef.current = {
      key,
      dx: e.clientX - rect.left - module.x * rect.width,
      dy: e.clientY - rect.top - module.y * rect.height,
    };
    previewRef.current?.setPointerCapture(e.pointerId);
    e.preventDefault();
  };

  const onDrag = (e: ReactPointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    const rect = previewRef.current?.getBoundingClientRect();
    if (!drag || !rect) return;
    const x = Math.min(0.98, Math.max(0, (e.clientX - rect.left - drag.dx) / rect.width));
    const y = Math.min(0.98, Math.max(0, (e.clientY - rect.top - drag.dy) / rect.height));
    patchModule(drag.key, { x, y });
  };

  const endDrag = () => {
    dragRef.current = null;
  };

  const resetHudPositions = () =>
    patchHud({
      fps: { ...draft.hud.fps, ...HUD_DEFAULT_POS.fps },
      cps: { ...draft.hud.cps, ...HUD_DEFAULT_POS.cps },
      keystrokes: { ...draft.hud.keystrokes, ...HUD_DEFAULT_POS.keystrokes },
      coords: { ...draft.hud.coords, ...HUD_DEFAULT_POS.coords },
      day: { ...draft.hud.day, ...HUD_DEFAULT_POS.day },
      ip: { ...draft.hud.ip, ...HUD_DEFAULT_POS.ip },
      armor: { ...draft.hud.armor, ...HUD_DEFAULT_POS.armor },
      fullbright: { ...draft.hud.fullbright, ...HUD_DEFAULT_POS.fullbright },
      time_changer: { ...draft.hud.time_changer, ...HUD_DEFAULT_POS.time_changer },
      ping: { ...draft.hud.ping, ...HUD_DEFAULT_POS.ping },
      toggle_sneak: { ...draft.hud.toggle_sneak, ...HUD_DEFAULT_POS.toggle_sneak },
      toggle_sprint: { ...draft.hud.toggle_sprint, ...HUD_DEFAULT_POS.toggle_sprint },
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
                <div className="mod-error">
                  <span className="mod-error__text">
                    <span className="mod-error__title">Mod download failed</span>
                    <span className="mod-error__body">
                      {modsState.error}. This version will run as vanilla.
                    </span>
                  </span>
                  <button
                    className="btn btn--ghost"
                    onClick={async () => {
                      const id = draft.selectedVersion;
                      if (!id) return;
                      await onClearModState(id);
                      onVerify();
                    }}
                    disabled={busy || !draft.selectedVersion}
                  >
                    Retry Mod Download
                  </button>
                </div>
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

          {tab === "hud" && (
            <>
              <div className="mod-row">
                <span>
                  <span className="mod-row__name">In-game HUD (Optional)</span>
                  <span className="mod-row__desc">
                    Custom overlay rendered by the bundled Diamond HUD mod (Fabric). Enable to use the in-game overlay.
                  </span>
                </span>
                <Switch label="Enable HUD" checked={draft.hud.enabled} onChange={(v) => patchHud({ enabled: v })} />
              </div>

              <div className="field">
                <div className="field__row">
                  <span className="field__label">Layout</span>
                  <button className="link" onClick={resetHudPositions} disabled={!draft.hud.enabled}>
                    Reset positions
                  </button>
                </div>
                <div
                  ref={previewRef}
                  className={`hud-preview ${draft.hud.enabled ? "" : "is-off"}`}
                  onPointerMove={onDrag}
                  onPointerUp={endDrag}
                  onPointerCancel={endDrag}
                  onPointerLeave={endDrag}
                >
                  <span className="hud-preview__hint">Drag to place modules</span>
                  {HUD_MODULES.map(([key, label]) => {
                    const module = draft.hud[key];
                    return (
                      <button
                        type="button"
                        key={key}
                        className={`hud-preview__chip ${module.enabled ? "" : "is-disabled"}`}
                        style={{ left: `${module.x * 100}%`, top: `${module.y * 100}%` }}
                        onPointerDown={(e) => startDrag(e, key)}
                        aria-label={`Move ${label} module`}
                      >
                        {label}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="hud-modules">
                {HUD_MODULES.map(([key, label]) => (
                  <div className="mod-row" key={key}>
                    <span>
                      <span className="mod-row__name">{label}</span>
                      <span className="mod-row__desc">{HUD_DESCRIPTIONS[key]}</span>
                    </span>
                    <Switch
                      label={`${label} module`}
                      checked={draft.hud[key].enabled}
                      disabled={!draft.hud.enabled}
                      onChange={(v) => patchModule(key, { enabled: v })}
                    />
                  </div>
                ))}
              </div>

              <div className="mod-row">
                <span>
                  <span className="mod-row__name">Fullbright</span>
                  <span className="mod-row__desc">
                    Rendering-level brightness (no slider): dark caves and nighttime stay clearly visible.
                    Restores normal lighting immediately when turned off.
                  </span>
                </span>
                <Switch
                  label="Fullbright module"
                  checked={draft.hud.fullbright.enabled}
                  onChange={(v) => patchModule("fullbright", { enabled: v })}
                />
              </div>

              {(() => {
                const extra: HudKey[] = ["time_changer", "ping", "toggle_sneak", "toggle_sprint"];
                return extra.map((key) => (
                  <div className="mod-row" key={key}>
                    <span>
                      <span className="mod-row__name">
                        {key === "time_changer" ? "Time Changer" : key === "ping" ? "Ping" : key === "toggle_sneak" ? "Toggle Sneak" : "Toggle Sprint"}
                      </span>
                      <span className="mod-row__desc">{HUD_DESCRIPTIONS[key]}</span>
                    </span>
                    <Switch
                      label={`${key} module`}
                      checked={draft.hud[key].enabled}
                      onChange={(v) => patchModule(key, { enabled: v })}
                    />
                  </div>
                ));
              })()}

              <div className="hud-settings">
                <label className="field">
                  <div className="field__row">
                    <span className="field__label">Scale</span>
                    <span className="field__value">{draft.hud.scale.toFixed(2)}×</span>
                  </div>
                  <input
                    type="range"
                    min={0.5}
                    max={2}
                    step={0.05}
                    value={draft.hud.scale}
                    disabled={!draft.hud.enabled}
                    onChange={(e) => patchHud({ scale: Number(e.target.value) })}
                  />
                </label>
                <label className="field">
                  <div className="field__row">
                    <span className="field__label">Panel opacity</span>
                    <span className="field__value">{Math.round(draft.hud.backgroundOpacity * 100)}%</span>
                  </div>
                  <input
                    type="range"
                    min={0}
                    max={1}
                    step={0.05}
                    value={draft.hud.backgroundOpacity}
                    disabled={!draft.hud.enabled}
                    onChange={(e) => patchHud({ backgroundOpacity: Number(e.target.value) })}
                  />
                </label>
                <div className="hud-colors">
                  <label className="field">
                    <span className="field__label">Accent</span>
                    <input
                      type="color"
                      value={draft.hud.accent}
                      disabled={!draft.hud.enabled}
                      onChange={(e) => patchHud({ accent: e.target.value })}
                    />
                  </label>
                  <label className="field">
                    <span className="field__label">Text</span>
                    <input
                      type="color"
                      value={draft.hud.textColor}
                      disabled={!draft.hud.enabled}
                      onChange={(e) => patchHud({ textColor: e.target.value })}
                    />
                  </label>
                </div>
              </div>

              <span className="field__hint">
                Available for Minecraft 26.2 and newer. The HUD runs on Fabric; Fabric API is installed automatically
                while the overlay is on, even if the performance mods are switched off. Changes apply the next time you
                press Play.
              </span>
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
