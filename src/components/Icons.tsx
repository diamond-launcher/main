import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const base: IconProps = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": true,
};

export const ChevronDown = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="m4 6 4 4 4-4" />
  </svg>
);

export const Check = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="m3.5 8.5 3 3 6-7" />
  </svg>
);

export const Diamond = (p: IconProps) => (
  <svg {...base} viewBox="0 0 24 24" strokeWidth={1.4} {...p}>
    <path d="M6 3h12l4 6-10 12L2 9z" />
    <path d="M2 9h20M9 3l-2 6 5 12 5-12-2-6" />
  </svg>
);

export const Gear = (p: IconProps) => (
  <svg {...base} viewBox="0 0 24 24" strokeWidth={1.8} {...p}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

export const Close = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="m4 4 8 8M12 4l-8 8" />
  </svg>
);

export const Minimize = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M4 8h8" />
  </svg>
);

export const Play = (p: IconProps) => (
  <svg {...base} fill="currentColor" stroke="none" {...p}>
    <path d="M4.5 2.8v10.4a.6.6 0 0 0 .9.5l8.2-5.2a.6.6 0 0 0 0-1L5.4 2.3a.6.6 0 0 0-.9.5Z" />
  </svg>
);

export const Download = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M8 2v8m0 0 3-3m-3 3L5 7M3 13h10" />
  </svg>
);
