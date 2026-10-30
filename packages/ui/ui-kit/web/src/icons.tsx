/** Inline icon set (no external icon CDN/font; stroke follows currentColor). */

export interface IconProps {
  className?: string;
}

function iconClass(className: string | undefined): string {
  return className === undefined ? "icon" : `icon ${className}`;
}

const BASE_PROPS = {
  width: 16,
  height: 16,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": true,
  focusable: false,
} as const;

export function IconPlus(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

export function IconSend(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M4 12l16-7-7 16-2.5-6.5L4 12z" />
    </svg>
  );
}

export function IconStop(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <rect x="7" y="7" width="10" height="10" rx="1.5" />
    </svg>
  );
}

export function IconSliders(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M4 7h10M18 7h2M4 17h2M10 17h10" />
      <circle cx="16" cy="7" r="2" />
      <circle cx="8" cy="17" r="2" />
    </svg>
  );
}

export function IconMenu(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M4 6h16M4 12h16M4 18h16" />
    </svg>
  );
}

export function IconClose(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M6 6l12 12M18 6L6 18" />
    </svg>
  );
}

export function IconRetry(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M20 12a8 8 0 1 1-2.34-5.66" />
      <path d="M20 4v4h-4" />
    </svg>
  );
}

export function IconChevron(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M6 9l6 6 6-6" />
    </svg>
  );
}

export function IconAlert(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M12 3l9 16H3l9-16z" />
      <path d="M12 9v5M12 17v.01" />
    </svg>
  );
}

export function IconWifiOff(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M3 3l18 18" />
      <path d="M8.5 15.5a5 5 0 0 1 7 0M5.5 12.5a10 10 0 0 1 3-1.9M18.5 12.5a10 10 0 0 0-2.2-1.6M2 9a15 15 0 0 1 5-3" />
      <path d="M12 19h.01" />
    </svg>
  );
}

export function IconBot(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <rect x="4" y="7" width="16" height="12" rx="3" />
      <path d="M12 4v3M9 13h.01M15 13h.01" />
    </svg>
  );
}

export function IconUser(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <circle cx="12" cy="8" r="4" />
      <path d="M4 21a8 8 0 0 1 16 0" />
    </svg>
  );
}

export function IconCheck(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M4 12l5 5L20 6" />
    </svg>
  );
}

export function IconCopy(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M6 15H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

export function IconSearch(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <circle cx="11" cy="11" r="6.5" />
      <path d="M16 16l4 4" />
    </svg>
  );
}

export function IconFolder(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M3.5 7.5a1.5 1.5 0 0 1 1.5-1.5h5l2 2h7a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 17.5v-10Z" />
    </svg>
  );
}

export function IconMore(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M6 12h.01M12 12h.01M18 12h.01" />
    </svg>
  );
}

export function IconCommand(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <path d="M9 6a2.5 2.5 0 1 0-2.5 2.5H9V6Zm0 0v10a2.5 2.5 0 1 1-2.5-2.5H18a2.5 2.5 0 1 1-2.5 2.5V6a2.5 2.5 0 1 1 2.5 2.5H9Z" />
    </svg>
  );
}

export function IconTask(props: IconProps) {
  return (
    <svg {...BASE_PROPS} className={iconClass(props.className)}>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <path d="M7 9l2 2 3-3M7 15h10" />
    </svg>
  );
}
