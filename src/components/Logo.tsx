/**
 * Rocktier letter mark: the product code (PE) on the app-icon tile, plus the
 * signature red dot. Keeping the tile dark in both themes means the mark in the
 * toolbar looks exactly like the icon in the Dock.
 */
export function Logo({ size = 24 }: { size?: number }) {
  return (
    <svg
      className="brand-mark"
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
    >
      <rect className="brand-tile" x="1" y="1" width="30" height="30" rx="8.5" />
      <text
        className="brand-letters"
        x="15.4"
        y="16.6"
        fontSize="15"
        fontWeight="700"
        letterSpacing="-1.1"
        textAnchor="middle"
        dominantBaseline="central"
      >
        PE
      </text>
      <circle className="brand-pip" cx="25.6" cy="6.4" r="2.1" />
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

export const IconCompress = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="5.2" y="5.2" width="5.6" height="5.6" rx="0.8" />
    <path d="M2 2.8 4.2 4.4M14 2.8 11.8 4.4M2 13.2 4.2 11.6M14 13.2 11.8 11.6" />
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

export const IconUndo = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M3 7.5h6.5a3.5 3.5 0 0 1 0 7H6" />
    <path d="M5.5 4.5 2.5 7.5l3 3" />
  </svg>
);

export const IconRedo = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M13 7.5H6.5a3.5 3.5 0 0 0 0 7H10" />
    <path d="M10.5 4.5l3 3-3 3" />
  </svg>
);

export const IconForm = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="2.5" y="3" width="11" height="10" rx="1.2" />
    <path d="M5 6h4M5 8.5h6M5 11h3" />
  </svg>
);

export const IconLock = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="3.5" y="7" width="9" height="6.5" rx="1.2" />
    <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
  </svg>
);

export const IconSign = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M2.5 11.5c2 0 3-1.1 4-3.3s2-4.2 3.1-4.2c1 0 1.2 1 1.5 2.4.3 1.5.7 2.4 1.7 2.4.5 0 .9-.2 1.2-.5" />
    <path d="M2.5 13.5h11" />
  </svg>
);

export const IconNote = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M3 3.5h10v6.5L9.5 14H3V3.5Z" />
    <path d="M12.5 10H9.5v3.5" />
    <path d="M5.5 6.3h5M5.5 8.4h3" />
  </svg>
);

export const IconHighlight = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M3 13.5h10" />
    <path d="M4.2 11 9.8 3.2a1.4 1.4 0 0 1 2-.4l1 .8a1.4 1.4 0 0 1 .4 2L7 13" />
  </svg>
);

export const IconUnderline = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M4.5 3v5a3.5 3.5 0 0 0 7 0V3" />
    <path d="M3 13.5h10" />
  </svg>
);

export const IconStrikeout = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M4.5 4.5c0-1 .9-1.7 3-1.7 1.6 0 2.6.5 3 1.3" />
    <path d="M11.5 11.5c0 1-.9 1.7-3 1.7-1.6 0-2.6-.5-3-1.3" />
    <path d="M3 8h10" />
  </svg>
);

export const IconExportImages = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="2.5" y="3.5" width="11" height="9" rx="1.2" />
    <path d="M2.5 10.2 5.4 7.8l2.4 1.8 2-1.5 3.2 2.3" />
    <circle cx="5.9" cy="6.3" r="1" />
  </svg>
);

export const IconImagesToPdf = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="2.5" y="2.5" width="7" height="8.5" rx="1" />
    <path d="M2.5 8.6 5 6.7l2.5 1.9" />
    <path d="M11 8.5v5M8.5 11h5" />
  </svg>
);

export const IconStamp = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="2.5" y="9.5" width="11" height="4" rx="1" />
    <path d="M4.5 9.5V6a3.5 3.5 0 0 1 7 0v3.5" />
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

export const IconCopy = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="5.5" y="5.5" width="8" height="8" rx="1.2" />
    <path d="M10.5 5.5V4A1.5 1.5 0 0 0 9 2.5H4A1.5 1.5 0 0 0 2.5 4v5A1.5 1.5 0 0 0 4 10.5h1.5" />
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
