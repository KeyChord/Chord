import { Tooltip as HeroTooltip } from "@heroui/react/tooltip";
import { cn } from "@chord/com.npmjs.utils-cn";
import type { ComponentProps, ReactElement, ReactNode } from "react";
import { cloneElement } from "react";

const Tooltip = HeroTooltip;
function TooltipTrigger({
  children,
}: {
  children: ReactElement<{ className?: string; children?: ReactNode }>;
}) {
  return (
    <HeroTooltip.Trigger
      tabIndex={0}
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
function TooltipContent({
  sideOffset = 4,
  ...props
}: ComponentProps<typeof HeroTooltip.Content> & { sideOffset?: number }) {
  return <HeroTooltip.Content {...props} offset={sideOffset} />;
}
export { Tooltip, TooltipTrigger, TooltipContent };
