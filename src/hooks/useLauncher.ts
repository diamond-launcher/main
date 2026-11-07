import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, events, toAppError } from "../api/tauri";
import type {
  AppError,
  GameStatus,
  InstallProgress,
  ModCatalogEntry,
  ModsState,
  Settings,
  SystemInfo,
  VersionInfo,
  VersionList,
} from "../api/types";

const RECOVERABLE_CODES = new Set(["not_installed", "java_missing", "mods_outdated"]);

export interface Launcher {
  ready: boolean;
  settings: Settings | null;
  system: SystemInfo | null;
  versions: VersionList | null;
  selected: VersionInfo | null;
  selectedId: string | null;
  showSnapshots: boolean;
  install: InstallProgress | null;
  game: GameStatus | null;
  error: AppError | null;
  modCatalog: ModCatalogEntry[];
  modsState: ModsState | null;
  isInstalling: boolean;
  isGameActive: boolean;
  dismissError: () => void;
  select: (id: string) => void;
  setShowSnapshots: (show: boolean) => void;
  installSelected: (verify?: boolean) => Promise<void>;
  play: () => Promise<void>;
  stop: () => Promise<void>;
  saveSettings: (next: Settings) => Promise<void>;
  reloadVersions: () => Promise<void>;
}

/** Single source of truth for launcher state; talks to the Rust backend only through `api`/`events`. */
export function useLauncher(): Launcher {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [system, setSystem] = useState<SystemInfo | null>(null);
  const [versions, setVersions] = useState<VersionList | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showSnapshots, setShowSnapshotsState] = useState(false);
  const [install, setInstall] = useState<InstallProgress | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [game, setGame] = useState<GameStatus | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [ready, setReady] = useState(false);
  const [modCatalog, setModCatalog] = useState<ModCatalogEntry[]>([]);
  const [modsState, setModsState] = useState<ModsState | null>(null);
  const selectedIdRef = useRef<string | null>(null);
  selectedIdRef.current = selectedId;

  const loadVersions = useCallback(async (preferred: string | null) => {
    try {
      const list = await api.listVersions();
      setVersions(list);
      setSelectedId((current) => {
        const wanted = current ?? preferred;
        return wanted && list.versions.some((v) => v.id === wanted) ? wanted : list.latestRelease;
      });
    } catch (e) {
      setError(toAppError(e));
    }
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisten: Array<() => void> = [];
    (async () => {
      const offProgress = await events.onInstallProgress((p) => {
        if (p.phase !== "done") setInstall(p);
      });
      const offGame = await events.onGameStatus((s) => {
        setGame(s);
        if (s.state === "running") void api.getSettings().then(setSettings);
        if (s.state === "crashed") setError({ code: "crashed", message: s.message ?? "The game crashed." });
      });
      if (disposed) {
        offProgress();
        offGame();
        return;
      }
      unlisten.push(offProgress, offGame);

      try {
        const [loaded, sys, running] = await Promise.all([api.getSettings(), api.systemInfo(), api.runningGame()]);
        if (disposed) return;
        setSettings(loaded);
        setSystem(sys);
        setShowSnapshotsState(loaded.showSnapshots);
        setSelectedId(loaded.selectedVersion);
        if (running) setGame({ version: running, state: "running", exitCode: null, message: null, logTail: [] });
        setReady(true);
        await loadVersions(loaded.selectedVersion ?? loaded.lastPlayed?.version ?? null);
      } catch (e) {
        setError(toAppError(e));
        setReady(true);
      }
    })();
    return () => {
      disposed = true;
      unlisten.forEach((off) => off());
    };
  }, [loadVersions]);

  const selected = useMemo(
    () => versions?.versions.find((v) => v.id === selectedId) ?? null,
    [versions, selectedId],
  );

  useEffect(() => {
    api.modCatalog().then(setModCatalog).catch(() => undefined);
  }, []);

  const installedFlag = selected?.installed ?? false;
  useEffect(() => {
    if (!selectedId) return;
    let stale = false;
    api
      .modsStatus(selectedId)
      .then((s) => !stale && setModsState(s))
      .catch(() => !stale && setModsState(null));
    return () => {
      stale = true;
    };
  }, [selectedId, installedFlag, isInstalling, settings]);

  const select = useCallback(
    (id: string) => {
      setSelectedId(id);
      setError(null);
      api.selectVersion(id, showSnapshots).catch((e) => setError(toAppError(e)));
    },
    [showSnapshots],
  );

  const setShowSnapshots = useCallback((show: boolean) => {
    setShowSnapshotsState(show);
    const id = selectedIdRef.current;
    if (id) api.selectVersion(id, show).catch(() => undefined);
  }, []);

  const runInstall = useCallback(async (id: string, verify: boolean) => {
    setIsInstalling(true);
    setInstall({ version: id, phase: "metadata", bytesDone: 0, bytesTotal: 0, filesDone: 0, filesTotal: 0 });
    try {
      await api.installVersion(id, verify);
      setVersions((list) =>
        list ? { ...list, versions: list.versions.map((v) => (v.id === id ? { ...v, installed: true } : v)) } : list,
      );
    } finally {
      setIsInstalling(false);
      setInstall(null);
    }
  }, []);

  const installSelected = useCallback(
    async (verify = false) => {
      if (!selectedId) return;
      setError(null);
      try {
        await runInstall(selectedId, verify);
      } catch (e) {
        setError(toAppError(e));
      }
    },
    [selectedId, runInstall],
  );

  const play = useCallback(async () => {
    if (!selectedId) return;
    setError(null);
    setGame(null);
    try {
      try {
        await api.launchGame(selectedId);
      } catch (e) {
        const err = toAppError(e);
        if (!RECOVERABLE_CODES.has(err.code)) throw err;
        await runInstall(selectedId, err.code !== "mods_outdated");
        await api.launchGame(selectedId);
      }
    } catch (e) {
      setError(toAppError(e));
      setGame(null);
    }
  }, [selectedId, runInstall]);

  const stop = useCallback(async () => {
    try {
      await api.stopGame();
    } catch (e) {
      setError(toAppError(e));
    }
  }, []);

  const saveSettings = useCallback(async (next: Settings) => {
    const saved = await api.saveSettings(next);
    setSettings(saved);
  }, []);

  const reloadVersions = useCallback(async () => {
    setError(null);
    await loadVersions(settings?.selectedVersion ?? null);
  }, [loadVersions, settings]);

  const isGameActive = game?.state === "starting" || game?.state === "running";

  return {
    ready,
    settings,
    system,
    versions,
    selected,
    selectedId,
    showSnapshots,
    install,
    game,
    error,
    modCatalog,
    modsState,
    isInstalling,
    isGameActive,
    dismissError: () => setError(null),
    select,
    setShowSnapshots,
    installSelected,
    play,
    stop,
    saveSettings,
    reloadVersions,
  };
}
