import { ProgressBar } from "./ProgressBar";

interface Props {
  version: string;
  busy: boolean;
  progress: { bytesDone: number; bytesTotal: number } | null;
  onUpdate: () => void;
  onLater: () => void;
}

export function UpdateBanner({ version, busy, progress, onUpdate, onLater }: Props) {
  const downloading = busy && progress != null;
  const percent = progress
    ? Math.round((100 * progress.bytesDone) / Math.max(1, progress.bytesTotal))
    : 0;

  return (
    <div className="update__backdrop" role="alert" aria-modal="true">
      <div className="update__card">
        <div className="update__header">
          <span className="update__title">Yeni Sürüm Mevcut</span>
          <span className="update__version">Diamond Launcher {version}</span>
        </div>
        {progress != null && (
          <div className="update__progress">
            <ProgressBar value={progress.bytesDone / Math.max(1, progress.bytesTotal)} />
            <span className="update__progress-text">{percent}%</span>
          </div>
        )}
        <div className="update__actions">
          <button className="btn btn--primary" onClick={onUpdate} disabled={busy}>
            {busy ? (downloading ? "İndiriliyor…" : "Güncelleniyor…") : "Güncelle"}
          </button>
          <button className="link" onClick={onLater} disabled={busy}>
            Sonra
          </button>
        </div>
      </div>
    </div>
  );
}
