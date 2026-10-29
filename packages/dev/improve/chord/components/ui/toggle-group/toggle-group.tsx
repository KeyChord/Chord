import { ToggleButtonGroup } from "@heroui/react/toggle-button-group";
import { ToggleButton } from "@heroui/react/toggle-button";
import type { ComponentProps } from "react";

function ToggleGroup({
  value,
  onValueChange,
  type: _type,
  variant: _variant,
  spacing = 0,
  size = "default",
  ...props
}: Omit<ComponentProps<typeof ToggleButtonGroup>, "selectedKeys" | "onSelectionChange" | "size"> & {
  type: "single";
  value?: string;
  onValueChange?: (value: string) => void;
  variant?: "outline" | "default";
  spacing?: number;
  size?: "default" | "sm";
}) {
  return (
    <ToggleButtonGroup
      {...props}
      selectionMode="single"
      disallowEmptySelection
      isDetached={spacing > 0}
      size={size === "sm" ? "sm" : "md"}
      selectedKeys={value ? [value] : []}
      onSelectionChange={(keys) => onValueChange?.(String([...keys][0] ?? ""))}
    />
  );
}
function ToggleGroupItem({
  value,
  ...props
}: ComponentProps<typeof ToggleButton> & { value: string }) {
  return <ToggleButton {...props} id={value} variant="ghost" />;
}
export { ToggleGroup, ToggleGroupItem };
