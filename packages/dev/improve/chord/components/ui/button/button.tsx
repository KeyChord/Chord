import { cn } from "@chord/com.npmjs.utils-cn";
import { Button as HeroButton } from "@heroui/react/button";
import type { ComponentProps } from "react";

const variants = {
  default: "primary",
  outline: "outline",
  secondary: "secondary",
  ghost: "ghost",
  destructive: "danger-soft",
  link: "ghost",
} as const;
const sizes = {
  default: "h-8 px-2.5",
  xs: "h-6 px-2 text-xs",
  sm: "h-7 px-2.5 text-[0.8rem]",
  lg: "h-9 px-3",
  icon: "size-8 p-0",
  "icon-xs": "size-6 p-0",
  "icon-sm": "size-7 p-0",
  "icon-lg": "size-9 p-0",
} as const;
type ButtonProps = Omit<ComponentProps<typeof HeroButton>, "variant" | "size" | "className"> & {
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
  disabled?: boolean;
  title?: string;
  className?: string;
};

/** Chord's compact desktop sizing on HeroUI's accessible button. */
function Button({
  className,
  variant = "default",
  size = "default",
  disabled,
  isDisabled,
  ...props
}: ButtonProps) {
  return (
    <HeroButton
      {...props}
      data-slot="button"
      data-variant={variant}
      data-size={size}
      variant={variants[variant]}
      size="sm"
      isIconOnly={size.startsWith("icon")}
      isDisabled={disabled ?? isDisabled}
      className={cn(
        "gap-1.5 rounded-lg",
        sizes[size],
        variant === "link" && "text-primary underline-offset-4 hover:underline",
        className,
      )}
    />
  );
}
export { Button };
export type { ButtonProps };
