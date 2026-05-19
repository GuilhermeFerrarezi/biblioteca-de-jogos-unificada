function SteamIcon({ size = 18, className, ...props }) {
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
        d="M12 2.25a9.75 9.75 0 0 0-9.68 8.62l5.24 2.16a2.74 2.74 0 0 1 1.63-.53h.15l2.33-3.38v-.05a3.67 3.67 0 1 1 3.67 3.67h-.08l-3.31 2.37v.16a2.77 2.77 0 0 1-5.47.55l-3.75-1.55A9.75 9.75 0 1 0 12 2.25Z"
        fill="currentColor"
      />
      <path
        d="M8.52 16.68a1.49 1.49 0 0 0 1.95-1.98 1.49 1.49 0 0 0-1.7-.76l.9.37a1.1 1.1 0 1 1-.84 2.03L8 16c.14.3.39.55.72.68Z"
        fill="currentColor"
      />
      <path
        d="M15.33 11.5a2.43 2.43 0 1 0 0-4.86 2.43 2.43 0 0 0 0 4.86Zm0-.68a1.75 1.75 0 1 1 0-3.5 1.75 1.75 0 0 1 0 3.5Z"
        fill="currentColor"
      />
    </svg>
  )
}

export default SteamIcon
