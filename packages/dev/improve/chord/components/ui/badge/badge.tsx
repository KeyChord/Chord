import { cn } from "@chord/com.npmjs.utils-cn";
import { Chip } from "@heroui/react/chip";
import type { ComponentProps } from "react";

function Badge({
  className,
  variant = "default",
  children,
  ...props
}: ComponentProps<"span"> & {
  variant?: "default" | "secondary" | "destructive" | "outline";
}) {
  return (
    <Chip
      {...props}
      data-slot="badge"
      size="sm"
      color={variant === "destructive" ? "danger" : variant === "default" ? "accent" : "default"}
      variant={variant === "outline" ? "primary" : "soft"}
      className={cn(
        "h-5 gap-1 px-2 text-xs",
        variant === "outline" && "border border-border bg-transparent text-foreground",
        className,
      )}
    >
      {children}
    </Chip>
  );
}
export { Badge };
