import { Disclosure } from "@heroui/react/disclosure";
import type { ComponentProps } from "react";

function Collapsible({
  open,
  defaultOpen,
  onOpenChange,
  ...props
}: Omit<
  ComponentProps<typeof Disclosure>,
  "isExpanded" | "defaultExpanded" | "onExpandedChange"
> & {
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
  return (
    <Disclosure
      {...props}
      isExpanded={open}
      defaultExpanded={defaultOpen}
      onExpandedChange={onOpenChange}
    />
  );
}
const CollapsibleTrigger = Disclosure.Trigger;
const CollapsibleContent = Disclosure.Content;
export { Collapsible, CollapsibleTrigger, CollapsibleContent };
