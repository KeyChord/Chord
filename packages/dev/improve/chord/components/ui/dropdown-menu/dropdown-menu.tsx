import { Dropdown } from "@heroui/react/dropdown";
import { Header } from "@heroui/react/header";
import { Separator } from "@heroui/react/separator";
import type { ComponentProps } from "react";

const DropdownMenu = Dropdown;
function DropdownMenuContent({
  align = "center",
  children,
  ...props
}: ComponentProps<typeof Dropdown.Popover> & { align?: "start" | "center" | "end" }) {
  return (
    <Dropdown.Popover {...props} placement={align === "center" ? "bottom" : `bottom ${align}`}>
      <Dropdown.Menu aria-label="Repository actions">{children}</Dropdown.Menu>
    </Dropdown.Popover>
  );
}
function DropdownMenuItem({
  disabled,
  variant,
  onSelect,
  ...props
}: Omit<ComponentProps<typeof Dropdown.Item>, "variant" | "onSelect"> & {
  disabled?: boolean;
  variant?: "destructive";
  onSelect?: () => void;
}) {
  return (
    <Dropdown.Item
      {...props}
      isDisabled={disabled}
      variant={variant === "destructive" ? "danger" : undefined}
      onAction={onSelect}
    />
  );
}
/** A section's title. Inside a menu it has to be a collection header, not a plain element. */
const DropdownMenuHeader = Header;
const DropdownMenuSeparator = Separator;
export {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuHeader,
  DropdownMenuItem,
  DropdownMenuSeparator,
};
