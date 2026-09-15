/** Rocktier wordmark: the diamond mark plus the signature red dot. */
export function Logo({ size = 22 }: { size?: number }) {
  return (
    <svg
      className="brand-mark"
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M16 2.5 29.5 16 16 29.5 2.5 16 16 2.5Z"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <path d="M16 8.5 23.5 16 16 23.5 8.5 16 16 8.5Z" fill="currentColor" opacity="0.18" />
      <circle cx="16" cy="16" r="2.4" fill="var(--red)" />
    </svg>
  );
}

/** Larger mark used on the empty state. */
export function DocumentGlyph() {
  return (
    <svg
      className="dropzone-icon"
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinejoin="round"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M12 4h16l10 10v30H12V4Z" />
      <path d="M28 4v10h10" />
      <path d="M18 26h14M18 32h14M18 38h9" opacity="0.55" />
    </svg>
  );
}

/** Inline control icons — keep them 16px, single-weight, monochrome. */
type IconProps = { size?: number };

const base = (size: number) => ({
  width: size,
  height: size,
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.4,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
  'aria-hidden': true,
});

export const IconOpen = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h2.2l1.1 1.4h5.7A1.5 1.5 0 0 1 14 5.9v5.6a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5v-7Z" />
  </svg>
);

export const IconSave = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M3 2.5h7.5L13 5v8.5H3V2.5Z" />
    <path d="M5.5 2.5v4h5v-4M5.5 13.5v-4h5v4" />
  </svg>
);

export const IconMerge = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M3 2.5v4a3 3 0 0 0 3 3h4a3 3 0 0 1 3 3v3" />
    <path d="M10.5 12.5 13 15.5 15.5 12.5" />
    <path d="M13 2.5v2.2a3 3 0 0 1-3 3H6" />
  </svg>
);

export const IconSplit = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M8 2.5v4M8 9.5v4" />
    <path d="M2.5 6.5 8 3l5.5 3.5" />
    <path d="M2.5 9.5 8 13l5.5-3.5" />
  </svg>
);

export const IconRotateLeft = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M2.5 8a5.5 5.5 0 1 1 1.8 4.1" />
    <path d="M2.5 4v4h4" />
  </svg>
);

export const IconRotateRight = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M13.5 8a5.5 5.5 0 1 0-1.8 4.1" />
    <path d="M13.5 4v4h-4" />
  </svg>
);

export const IconTrash = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4" />
  </svg>
);

export const IconExtract = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M8 2.5v7M8 9.5 5 7M8 9.5 11 7" />
    <path d="M2.5 11.5v2h11v-2" />
  </svg>
);

export const IconPlus = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M8 3v10M3 8h10" />
  </svg>
);

export const IconMinus = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M3 8h10" />
  </svg>
);

export const IconClose = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M4 4l8 8M12 4l-8 8" />
  </svg>
);

export const IconMoon = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M13 9.7A5.5 5.5 0 0 1 6.3 3a5.5 5.5 0 1 0 6.7 6.7Z" />
  </svg>
);

export const IconSun = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <circle cx="8" cy="8" r="3" />
    <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4" />
  </svg>
);
