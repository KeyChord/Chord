import { cn } from "@chord/com.npmjs.utils-cn";
import { Kbd as HeroKbd } from "@heroui/react/kbd";
import type { ComponentProps } from "react";

function Kbd({ children, className, ...props }: ComponentProps<"kbd">) {
  return (
    <HeroKbd
      {...props}
      data-slot="kbd"
      className={cn("h-5 min-w-5 justify-center rounded-sm px-1 font-sans text-xs", className)}
    >
      {children}
    </HeroKbd>
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
