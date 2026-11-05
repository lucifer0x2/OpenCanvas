const paths = {
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
  info: (
    <>
      <path d="M12 22C6.47715 22 2 17.5228 2 12 2 6.47715 6.47715 2 12 2 17.5228 2 22 6.47715 22 12 22 17.5228 17.5228 22 12 22ZM12 20C16.4183 20 20 16.4183 20 12 20 7.58172 16.4183 4 12 4 7.58172 4 4 7.58172 4 12 4 16.4183 7.58172 20 12 20ZM13 10.5V15H14V17H10V15H11V12.5H10V10.5H13ZM13.5 8C13.5 8.82843 12.8284 9.5 12 9.5 11.1716 9.5 10.5 8.82843 10.5 8 10.5 7.17157 11.1716 6.5 12 6.5 12.8284 6.5 13.5 7.17157 13.5 8Z"></path>
    </>
  ),
  riinfo: (
    <>
      <path d="M2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12ZM16.9056 18.3199C15.551 19.3729 13.8487 20 12 20C7.58172 20 4 16.4183 4 12C4 10.1513 4.62708 8.44904 5.68014 7.09436L10 11.4142V12.5H11V15H10V17H14V15.4142L16.9056 18.3199ZM18.3199 16.9056L13 11.5858V10.5H11.9142L7.09436 5.68014C8.44904 4.62708 10.1513 4 12 4C16.4183 4 20 7.58172 20 12C20 13.8487 19.3729 15.551 18.3199 16.9056ZM13 14.4142V15H13.5858L13 14.4142ZM12 9.5C12.8284 9.5 13.5 8.82843 13.5 8C13.5 7.17157 12.8284 6.5 12 6.5C11.1716 6.5 10.5 7.17157 10.5 8C10.5 8.82843 11.1716 9.5 12 9.5Z"></path>
    </>
  )
} as const;

export type IconName = keyof typeof paths

export type IconProps = { name: IconName; size?: number } & React.SVGProps<SVGSVGElement>

export function Icon({ name, size = 16, ...svgProps }: IconProps) {
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
      {...svgProps}
    >
      {paths[name] ?? paths.rect}
    </svg>
  );
}
