export function Icon({ name, size = 16 }: { name: string; size?: number }) {
  const paths: Record<string, React.ReactNode> = {
    layers: (
      <>
        <path d="m3 6 5-3 5 3-5 3-5-3Z" />
        <path d="m3 9 5 3 5-3M3 12l5 3 5-3" />
      </>
    ),
    frame: (
      <>
        <path d="M5 2v12M11 2v12M2 5h12M2 11h12" />
      </>
    ),
    text: (
      <>
        <path d="M3 3h10M8 3v10M5 13h6" />
      </>
    ),
    image: (
      <>
        <rect x="2.5" y="2.5" width="11" height="11" rx="1" />
        <path d="m3 11 3-4 3 3 2-2 2 3" />
        <circle cx="10.5" cy="5.5" r=".5" />
      </>
    ),
    rect: <rect x="3" y="3" width="10" height="10" rx="1" />,
    chevron: <path d="m6 4 4 4-4 4" />,
    panelLeft: (
      <>
        <rect x="2" y="3" width="12" height="10" rx="1" />
        <path d="M6 3v10" />
      </>
    ),
    panelRight: (
      <>
        <rect x="2" y="3" width="12" height="10" rx="1" />
        <path d="M10 3v10" />
      </>
    ),
    fit: (
      <>
        <path d="M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4" />
        <rect x="5" y="5" width="6" height="6" />
      </>
    ),
    minus: <path d="M3 8h10" />,
    plus: <path d="M3 8h10M8 3v10" />,
    refresh: (
      <>
        <path d="M13 6a5 5 0 1 0 .2 4M13 2v4H9" />
      </>
    ),
    page: (
      <>
        <path d="M4 2h5l3 3v9H4V2Z" />
        <path d="M9 2v4h3" />
      </>
    ),
  };
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.35"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[name] ?? paths.rect}
    </svg>
  );
}
