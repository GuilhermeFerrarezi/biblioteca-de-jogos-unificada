function EpicIcon({ size = 18, className, ...props }) {
  return (
    <svg
      aria-hidden="true"
      className={className}
      fill="none"
      height={size}
      viewBox="0 0 24 24"
      width={size}
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M4.25 2.25h15.5v12.38c0 3.73-2.2 6.2-7.75 8.12-5.55-1.92-7.75-4.39-7.75-8.12V2.25Z"
        fill="var(--epic-icon-shield, #050505)"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.25"
      />
      <text
        fill="currentColor"
        fontFamily="Arial Black, Impact, sans-serif"
        fontSize="5.15"
        fontWeight="900"
        letterSpacing="-.1"
        textAnchor="middle"
        x="12"
        y="10.25"
      >
        EPIC
      </text>
      <path d="M7.1 12.1h9.8" stroke="currentColor" strokeLinecap="round" strokeWidth="1.05" />
      <text
        fill="currentColor"
        fontFamily="Arial Black, Impact, sans-serif"
        fontSize="3.15"
        fontWeight="900"
        letterSpacing=".05"
        textAnchor="middle"
        x="12"
        y="16.15"
      >
        GAMES
      </text>
    </svg>
  )
}

export default EpicIcon
