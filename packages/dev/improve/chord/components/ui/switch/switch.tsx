import { cn } from "@chord/com.npmjs.utils-cn";
import { Switch as HeroSwitch } from "@heroui/react/switch";
import type { ComponentProps } from "react";

type SwitchProps = Omit<
  ComponentProps<typeof HeroSwitch>,
  "size" | "className" | "children" | "isSelected" | "defaultSelected" | "isDisabled" | "onChange"
> & {
  checked?: boolean;
  defaultChecked?: boolean;
  disabled?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  size?: "sm" | "default";
  className?: string;
};
function Switch({
  id,
  className,
  checked,
  defaultChecked,
  disabled,
  onCheckedChange,
  size = "default",
  "aria-label": ariaLabel,
  "aria-labelledby": labelledBy,
  "aria-describedby": describedBy,
  ...props
}: SwitchProps) {
  return (
    <HeroSwitch
      {...props}
      id={id}
      aria-label={ariaLabel}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      isSelected={checked}
      defaultSelected={defaultChecked}
      isDisabled={disabled}
      onChange={onCheckedChange}
      size={size === "default" ? "md" : "sm"}
      className={cn("shrink-0", className)}
    >
      <HeroSwitch.Content>
        <HeroSwitch.Control>
          <HeroSwitch.Thumb />
        </HeroSwitch.Control>
      </HeroSwitch.Content>
    </HeroSwitch>
  );
}
export { Switch };
