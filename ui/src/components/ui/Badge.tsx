import { HTMLAttributes } from "react";

type RoleBadge = "host" | "creator" | "moderator" | "member";
type StatusBadge = "free" | "yours" | "expired";
type BadgeVariant = RoleBadge | StatusBadge;

interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant: BadgeVariant;
  size?: "sm" | "md";
}

const variantStyles: Record<BadgeVariant, string> = {
  host: "bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-300",
  creator: "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300",
  moderator:
    "bg-purple-100 text-purple-800 dark:bg-purple-900/40 dark:text-purple-300",
  member: "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
  free: "bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300",
  yours: "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300",
  expired: "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300",
};

const variantLabels: Record<BadgeVariant, string> = {
  host: "Host",
  creator: "Creator",
  moderator: "Moderator",
  member: "Member",
  free: "Free",
  yours: "Yours",
  expired: "Expired",
};

const sizeClasses = {
  sm: "px-1.5 py-0.5 text-[10px]",
  md: "px-2 py-0.5 text-xs",
};

export default function Badge({
  variant,
  size = "md",
  className = "",
  children,
  ...props
}: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center font-semibold rounded-full whitespace-nowrap
        ${variantStyles[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children ?? variantLabels[variant]}
    </span>
  );
}
