import { getCurrentWindow } from "@tauri-apps/api/window";
import { Close, Minimize } from "./Icons";

export function TitleBar() {
  const win = getCurrentWindow();
  return (
    <header className="titlebar" data-tauri-drag-region>
      <span className="titlebar__name" data-tauri-drag-region>
        Diamond
      </span>
      <div className="titlebar__controls">
        <button className="win-btn" aria-label="Minimize" onClick={() => void win.minimize()}>
          <Minimize />
        </button>
        <button className="win-btn win-btn--close" aria-label="Close" onClick={() => void win.close()}>
          <Close />
        </button>
      </div>
    </header>
  );
}
