interface Props {
  /** 0..1, or `null` for an indeterminate (unmeasurable) phase. */
  value: number | null;
}

export function ProgressBar({ value }: Props) {
  const determinate = value !== null;
  return (
    <div
      className={`progress ${determinate ? "" : "progress--indeterminate"}`}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={determinate ? Math.round(value * 100) : undefined}
    >
      <div className="progress__fill" style={determinate ? { transform: `scaleX(${Math.min(1, value)})` } : undefined} />
    </div>
  );
}
