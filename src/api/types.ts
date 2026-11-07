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
