import type { InstallProgress } from "../api/types";
import { formatBytes } from "../lib/format";
import { ProgressBar } from "./ProgressBar";

const PHASE_LABELS: Record<string, string> = {
  metadata: "Fetching version data",
  java: "Preparing Java runtime",
  verify: "Verifying files",
  finalize: "Finishing up",
};

interface Props {
  install: InstallProgress;
}

/** Shows only measured progress; phases without a byte count render as indeterminate. */
export function InstallStatus({ install }: Props) {
  const downloading = install.phase === "download";
  const fraction = downloading && install.bytesTotal > 0 ? install.bytesDone / install.bytesTotal : null;
  const label = downloading
    ? `Downloading ${formatBytes(install.bytesDone)} of ${formatBytes(install.bytesTotal)}`
    : (PHASE_LABELS[install.phase] ?? "Working");

  return (
    <div className="install">
      <ProgressBar value={fraction} />
      <div className="install__meta">
        <span>{label}</span>
        {fraction !== null && <span>{Math.floor(fraction * 100)}%</span>}
      </div>
    </div>
  );
}
