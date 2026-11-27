import { cn } from "@chord/com.npmjs.utils-cn";
import type { SelectHTMLAttributes } from "react";

type SelectProps = Omit<
  SelectHTMLAttributes<HTMLSelectElement>,
  "value" | "onChange" | "children" | "disabled"
> & {
  value: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
  options: readonly { value: string; label: string }[];
};

/**
 * A pop-up button. It is a native `<select>`, so macOS opens its own menu (with the checkmark,
 * type-to-select and keyboard handling) instead of a web imitation.
 */
function Select({ value, onValueChange, disabled, options, className, ...props }: SelectProps) {
  return (
    <select
      {...props}
      data-slot="select"
      className={cn("mac-popup", className)}
      value={value}
      disabled={disabled}
      onChange={(event) => onValueChange(event.target.value)}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
export { Select };
