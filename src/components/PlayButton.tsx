import { Download, Play } from "./Icons";

export type PlayMode = "play" | "install" | "busy" | "running" | "disabled";

interface Props {
  mode: PlayMode;
  onClick: () => void;
}

const LABELS: Record<PlayMode, string> = {
  play: "Play",
  install: "Install",
  busy: "Installing",
  running: "Playing",
  disabled: "Play",
};

export function PlayButton({ mode, onClick }: Props) {
  const inactive = mode === "busy" || mode === "running" || mode === "disabled";
  return (
    <button className={`play play--${mode}`} onClick={onClick} disabled={inactive} aria-live="polite">
      {mode === "install" ? <Download width={20} height={20} /> : mode === "busy" ? null : <Play width={18} height={18} />}
      <span>{LABELS[mode]}</span>
    </button>
  );
}
