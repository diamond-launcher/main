import { ProgressBar } from "./ProgressBar";

interface Props {
  version: string;
  notes: string;
  busy: boolean;
  progress: { bytesDone: number; bytesTotal: number } | null;
  onUpdate: () => void;
  onLater: () => void;
}

export function UpdateBanner({ version, notes, busy, progress, onUpdate, onLater }: Props) {
  const downloading = busy && progress != null;
  const percent = progress
    ? Math.round((100 * progress.bytesDone) / Math.max(1, progress.bytesTotal))
    : 0;

  return (
    <div className="update" role="alert">
      <div className="update__card">
        <span className="update__text">Diamond Launcher {version} is available</span>
        {notes.length > 0 && (
          <span className="update__notes">{notes}</span>
        )}
        {progress != null && (
          <div className="update__progress">
            <ProgressBar value={progress.bytesDone / Math.max(1, progress.bytesTotal)} />
            <span className="update__progress-text">{percent}%</span>
          </div>
        )}
        <button className="btn btn--primary" onClick={onUpdate} disabled={busy}>
          {busy ? (downloading ? "Downloading…" : "Updating…") : "Update now"}
        </button>
        <button className="link" onClick={onLater} disabled={busy}>
          Later
        </button>
      </div>
    </div>
  );
}
