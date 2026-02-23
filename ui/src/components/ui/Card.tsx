import { forwardRef } from "react";
import { motion, type HTMLMotionProps } from "framer-motion";

type Padding = "none" | "sm" | "md" | "lg";

interface CardProps extends Omit<HTMLMotionProps<"div">, "ref" | "padding"> {
  padding?: Padding;
  hoverable?: boolean;
}

const paddingClasses: Record<Padding, string> = {
  none: "p-0",
  sm: "p-3",
  md: "p-4",
  lg: "p-6",
};

const Card = forwardRef<HTMLDivElement, CardProps>(
  (
    {
      padding = "md",
      hoverable = false,
      onClick,
      className = "",
      children,
      ...props
    },
    ref,
  ) => {
    const isClickable = !!onClick || hoverable;

    return (
      <motion.div
        ref={ref}
        whileHover={isClickable ? { y: -2, scale: 1.005 } : undefined}
        transition={{ type: "spring", stiffness: 300, damping: 25 }}
        onClick={onClick}
        className={`rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]
          ${isClickable ? "cursor-pointer" : ""}
          ${paddingClasses[padding]} ${className}`}
        {...props}
      >
        {children}
      </motion.div>
    );
  },
);

Card.displayName = "Card";
export default Card;
