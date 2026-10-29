import { Popover as HeroPopover } from "@heroui/react/popover";
import { cn } from "@chord/com.npmjs.utils-cn";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { cloneElement } from "react";

const Popover = HeroPopover;
function PopoverTrigger({
  children,
}: {
  children: ReactElement<{ className?: string; children?: ReactNode }>;
}) {
  return (
    <HeroPopover.Trigger
      render={(props) =>
        cloneElement(children, {
          ...props,
          className: cn(children.props.className, props.className),
          children: children.props.children,
        })
      }
    />
  );
}
function PopoverContent({
  children,
  align = "center",
  side = "bottom",
  sideOffset = 4,
  className,
  ...props
}: Omit<ComponentProps<typeof HeroPopover.Content>, "children"> & {
  children: ReactNode;
  align?: "start" | "center" | "end";
  side?: "top" | "bottom";
  sideOffset?: number;
}) {
  const placement = align === "center" ? side : (`${side} ${align}` as const);
  return (
    <HeroPopover.Content {...props} placement={placement} offset={sideOffset} className={className}>
      <HeroPopover.Dialog aria-label="Setting information">{children}</HeroPopover.Dialog>
    </HeroPopover.Content>
  );
}
export { Popover, PopoverTrigger, PopoverContent };
