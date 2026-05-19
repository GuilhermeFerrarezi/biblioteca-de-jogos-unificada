function XboxIcon({ size = 18, className, ...props }) {
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
        d="M12 2.25a9.75 9.75 0 0 0-6.15 2.18c1.66.08 3.91 1.44 6.15 3.62 2.24-2.18 4.49-3.54 6.15-3.62A9.75 9.75 0 0 0 12 2.25Z"
        fill="currentColor"
      />
      <path
        d="M4.63 5.64a9.75 9.75 0 0 0 1.08 13.82c-.38-2.75 1.19-6.7 4.4-10.27C7.77 6.96 5.66 5.68 4.63 5.64Z"
        fill="currentColor"
      />
      <path
        d="M19.37 5.64c-1.03.04-3.14 1.32-5.48 3.55 3.21 3.57 4.78 7.52 4.4 10.27a9.75 9.75 0 0 0 1.08-13.82Z"
        fill="currentColor"
      />
      <path
        d="M12 10.92c-3.66 3.72-5.36 7.86-4.58 9.88a9.75 9.75 0 0 0 9.16 0c.78-2.02-.92-6.16-4.58-9.88Z"
        fill="currentColor"
      />
    </svg>
  )
}

export default XboxIcon
