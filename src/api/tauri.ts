import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppError,
  GameStatus,
  InstallProgress,
  ModCatalogEntry,
  ModsState,
  Settings,
  SystemInfo,
  VersionList,
} from "./types";

/** Normalises anything thrown by `invoke` into an `AppError`. */
export function toAppError(e: unknown): AppError {
  if (typeof e === "object" && e !== null && "message" in e && "code" in e) {
    return e as AppError;
  }
  return { code: "unknown", message: typeof e === "string" ? e : "Something went wrong." };
}

export const api = {
  systemInfo: () => invoke<SystemInfo>("get_system_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  selectVersion: (id: string, showSnapshots: boolean) =>
    invoke<void>("select_version", { id, showSnapshots }),
  listVersions: () => invoke<VersionList>("list_versions"),
  installVersion: (id: string, verify: boolean) => invoke<void>("install_version", { id, verify }),
  launchGame: (id: string) => invoke<void>("launch_game", { id }),
  stopGame: () => invoke<void>("stop_game"),
  modCatalog: () => invoke<ModCatalogEntry[]>("list_mod_catalog"),
  modsStatus: (id: string) => invoke<ModsState | null>("mods_status", { id }),
  runningGame: () => invoke<string | null>("is_game_running"),
};

export const events = {
  onInstallProgress: (cb: (p: InstallProgress) => void): Promise<UnlistenFn> =>
    listen<InstallProgress>("install-progress", (e) => cb(e.payload)),
  onGameStatus: (cb: (s: GameStatus) => void): Promise<UnlistenFn> =>
    listen<GameStatus>("game-status", (e) => cb(e.payload)),
};
