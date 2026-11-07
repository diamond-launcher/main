import { useState } from "react";
import type { AppError } from "../api/types";

interface Props {
  error: AppError;
  logTail: string[];
  onDismiss: () => void;
  onRetry?: () => void;
}

export function ErrorNotice({ error, logTail, onDismiss, onRetry }: Props) {
  const [showLog, setShowLog] = useState(false);
  return (
    <div className="notice" role="alert">
      <p className="notice__message">{error.message}</p>
      <div className="notice__actions">
        {onRetry && (
          <button className="link" onClick={onRetry}>
            Retry
          </button>
        )}
        {logTail.length > 0 && (
          <button className="link" onClick={() => setShowLog((s) => !s)}>
            {showLog ? "Hide log" : "Show log"}
          </button>
        )}
        <button className="link" onClick={onDismiss}>
          Dismiss
        </button>
      </div>
      {showLog && <pre className="notice__log">{logTail.join("\n")}</pre>}
    </div>
  );
}
