import { cn } from "@chord/com.npmjs.utils-cn";
import { Input as HeroInput } from "@heroui/react/input";
import type { ComponentProps } from "react";

function Input({
  className,
  ...props
}: Omit<ComponentProps<typeof HeroInput>, "className"> & { className?: string }) {
  return (
    <HeroInput
      {...props}
      data-slot="input"
      className={cn("h-8 w-full min-w-0 rounded-lg px-2.5 py-1 text-sm", className)}
    />
  );
}
export { Input };
