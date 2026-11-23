import { cn } from "@chord/com.npmjs.utils-cn";
import type { ComponentProps } from "react";

/**
 * HeroUI's keycap as plain markup: its `Kbd` only adds the `kbd` class that
 * `@heroui/styles` styles, and importing it brings HeroUI's component runtime into the
 * Hint Panel's page.
 */
function Kbd({ children, className, ...props }: ComponentProps<"kbd">) {
  return (
    <kbd
      {...props}
      data-slot="kbd"
      className={cn("kbd", "h-5 min-w-5 justify-center rounded-sm px-1 font-mono text-xs", className)}
    >
      {children}
    </kbd>
  );
}
function KbdGroup({ className, ...props }: ComponentProps<"span">) {
  return (
    <span
      {...props}
      data-slot="kbd-group"
      className={cn("inline-flex items-center gap-1", className)}
    />
  );
}
export { Kbd, KbdGroup };
