import { Select as HeroSelect } from "@heroui/react/select";
import { ListBox } from "@heroui/react/list-box";
import type { ComponentProps } from "react";

type SelectProps = Omit<
  ComponentProps<typeof HeroSelect>,
  "children" | "value" | "onChange" | "items"
> & {
  value: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
  options: readonly { value: string; label: string }[];
};
function Select({ value, onValueChange, disabled, options, ...props }: SelectProps) {
  return (
    <HeroSelect
      {...props}
      value={value}
      onChange={(key) => {
        if (key !== null) onValueChange(String(key));
      }}
      isDisabled={disabled}
    >
      <HeroSelect.Trigger className="h-8 min-h-8 min-w-28 rounded-lg px-2.5 py-1 text-sm">
        <HeroSelect.Value />
        <HeroSelect.Indicator />
      </HeroSelect.Trigger>
      <HeroSelect.Popover>
        <ListBox aria-label={props["aria-label"]}>
          {options.map((option) => (
            <ListBox.Item key={option.value} id={option.value} textValue={option.label}>
              {option.label}
            </ListBox.Item>
          ))}
        </ListBox>
      </HeroSelect.Popover>
    </HeroSelect>
  );
}
export { Select };
