export type VersionKind = "release" | "snapshot";

export interface VersionInfo {
  id: string;
  kind: VersionKind;
  releaseTime: string;
  installed: boolean;
}

export interface VersionList {
  latestRelease: string;
  latestSnapshot: string;
  versions: VersionInfo[];
  offline: boolean;
}

export interface LastPlayed {
  version: string;
  /** Unix timestamp, seconds. */
  at: number;
}

export interface HudModule {
  enabled: boolean;
  /** Fraction of the screen width, 0..1. */
  x: number;
  /** Fraction of the screen height, 0..1. */
  y: number;
}

export interface HudSettings {
  enabled: boolean;
  /** 0.5..2.0 */
  scale: number;
  /** 0..1 */
  backgroundOpacity: number;
  /** #rrggbb */
  accent: string;
  /** #rrggbb */
  textColor: string;
  /** GLFW key code that opens the in-game HUD editor. */
  keybind: number;
  /** 0..3 */
  cornerRadius: number;
  fps: HudModule;
  cps: HudModule;
  keystrokes: HudModule;
  coords: HudModule;
  day: HudModule;
  ip: HudModule;
  armor: HudModule;
  fullbright: HudModule;
  timeChanger: HudModule;
  /** Time of day used by the Time Changer module (0..23999 overworld clock ticks). */
  timeChangerTime: number;
  ping: HudModule;
  toggleSprint: HudModule;
}

export interface Settings {
  username: string;
  maxRamMb: number | null;
  javaPath: string | null;
  gameDir: string | null;
  jvmArgs: string;
  selectedVersion: string | null;
  lastPlayed: LastPlayed | null;
  showSnapshots: boolean;
  modsEnabled: boolean;
  disabledMods: string[];
  hud: HudSettings;
}

export interface ModCatalogEntry {
  slug: string;
  name: string;
  description: string;
  category: string;
}

export interface ModsState {
  signature: string;
  profileId: string | null;
  files: string[];
  mods: string[];
  skipped: string[];
  error: string | null;
}

export interface JavaInstall {
  path: string;
  major: number;
}

export interface SystemInfo {
  os: string;
  arch: string;
  totalRamMb: number;
  availableRamMb: number;
  recommendedRamMb: number;
  defaultGameDir: string;
  systemJava: JavaInstall | null;
}

export type InstallPhase = "metadata" | "java" | "verify" | "download" | "finalize" | "done";

export interface InstallProgress {
  version: string;
  phase: InstallPhase;
  bytesDone: number;
  bytesTotal: number;
  filesDone: number;
  filesTotal: number;
}

export type GameState = "starting" | "running" | "exited" | "crashed";

export interface GameStatus {
  version: string;
  state: GameState;
  exitCode: number | null;
  message: string | null;
  logTail: string[];
}

export interface AppError {
  code: string;
  message: string;
}

export interface UpdateInfo {
  /** Version without the leading "v", e.g. "0.3.0". */
  version: string;
}

export interface AppInfo {
  version: string;
  developer: string;
  website: string;
}
