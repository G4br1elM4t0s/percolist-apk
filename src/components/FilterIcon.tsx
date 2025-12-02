interface FilterIconProps {
  className?: string
}

export function FilterIcon({ className }: FilterIconProps) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        d="M0 0L5.6 8.72002V16H10.4V8.72002L16 0H0ZM8.8 14.4H7.2V12.8H8.8V14.4ZM8.8 8V11.2H7.2V8L2.8 1.6H13.28L8.8 8Z"
        fill="currentColor"
      />
    </svg>
  )
}
