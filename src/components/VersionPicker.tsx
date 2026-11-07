import { useEffect, useMemo, useRef, useState } from "react";
import type { VersionInfo, VersionList } from "../api/types";
import { formatDate } from "../lib/format";
import { Check, ChevronDown } from "./Icons";

interface Props {
  versions: VersionList | null;
  selected: VersionInfo | null;
  showSnapshots: boolean;
  disabled: boolean;
  onSelect: (id: string) => void;
  onShowSnapshots: (show: boolean) => void;
}

export function VersionPicker({ versions, selected, showSnapshots, disabled, onSelect, onShowSnapshots }: Props) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const selectedRowRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointer = (e: PointerEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    selectedRowRef.current?.scrollIntoView({ block: "center" });
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const visible = useMemo(
    () => (versions?.versions ?? []).filter((v) => (showSnapshots ? v.kind === "snapshot" : v.kind === "release")),
    [versions, showSnapshots],
  );

  return (
    <div className="picker" ref={rootRef}>
      <button
        className="picker__trigger"
        onClick={() => setOpen((o) => !o)}
        disabled={disabled || !versions}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <span>Change version</span>
        <ChevronDown className={open ? "flip" : ""} />
      </button>

      {open && (
        <div className="picker__panel" role="dialog" aria-label="Select Minecraft version">
          <div className="segmented" role="tablist">
            <button role="tab" aria-selected={!showSnapshots} onClick={() => onShowSnapshots(false)}>
              Releases
            </button>
            <button role="tab" aria-selected={showSnapshots} onClick={() => onShowSnapshots(true)}>
              Snapshots
            </button>
          </div>
          <ul className="picker__list" role="listbox">
            {visible.map((v) => {
              const active = v.id === selected?.id;
              return (
                <li key={v.id}>
                  <button
                    ref={active ? selectedRowRef : undefined}
                    className={`picker__row ${active ? "is-active" : ""}`}
                    role="option"
                    aria-selected={active}
                    onClick={() => {
                      onSelect(v.id);
                      setOpen(false);
                    }}
                  >
                    <span className="picker__id">{v.id}</span>
                    {versions?.latestRelease === v.id && <span className="tag">Latest</span>}
                    {v.installed && <span className="tag tag--installed">Installed</span>}
                    <span className="picker__date">{formatDate(v.releaseTime)}</span>
                    <span className="picker__check">{active && <Check />}</span>
                  </button>
                </li>
              );
            })}
            {visible.length === 0 && <li className="picker__empty">No versions available.</li>}
          </ul>
        </div>
      )}
    </div>
  );
}
