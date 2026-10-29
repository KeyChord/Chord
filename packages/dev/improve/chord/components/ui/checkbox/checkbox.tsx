import { Checkbox as HeroCheckbox } from "@heroui/react/checkbox";
import { Label } from "@heroui/react/label";
import type { ComponentProps, ReactNode } from "react";

function Checkbox({
  checked,
  disabled,
  onCheckedChange,
  children,
  ...props
}: Omit<
  ComponentProps<typeof HeroCheckbox>,
  "children" | "isSelected" | "isDisabled" | "onChange"
> & {
  checked: boolean;
  disabled?: boolean;
  onCheckedChange: (checked: boolean) => void;
  /** Omit for a control labelled by `aria-label`, such as a table row selector. */
  children?: ReactNode;
}) {
  return (
    <HeroCheckbox {...props} isSelected={checked} isDisabled={disabled} onChange={onCheckedChange}>
      <HeroCheckbox.Content>
        <HeroCheckbox.Control>
          <HeroCheckbox.Indicator />
        </HeroCheckbox.Control>
        {children != null && <Label>{children}</Label>}
      </HeroCheckbox.Content>
    </HeroCheckbox>
  );
}
export { Checkbox };
