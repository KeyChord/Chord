import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  DropdownMenu,
  DropdownMenuItem,
} from "@chord/dev.improve.chord.components.ui.dropdown-menu";
import { ToggleGroup, ToggleGroupItem } from "@chord/dev.improve.chord.components.ui.toggle-group";
import { Check, Ellipsis } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";

export function CategoryFilter({
  options,
  value,
  onChange,
}: {
  options: { value: string; label: string }[];
  value: string;
  onChange: (value: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rulerRef = useRef<HTMLDivElement>(null);
  const [visibleCount, setVisibleCount] = useState(options.length);

  useLayoutEffect(() => {
    const container = containerRef.current!;
    const ruler = rulerRef.current!;
    const buttons = [...ruler.querySelectorAll("button")];
    const moreButton = buttons.pop()!;

    function measure() {
      const available = container.getBoundingClientRect().width;
      const start = ruler.getBoundingClientRect().left;
      const ends = buttons.map((button) => button.getBoundingClientRect().right - start);
      const fullWidth = ends.at(-1) ?? 0;
      const moreWidth = moreButton.getBoundingClientRect().right - start - fullWidth;
      const fittingCount = fullWidth <= available
        ? buttons.length
        : ends.filter((end) => end + moreWidth <= available).length;
      setVisibleCount(fittingCount);
    }

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(container);
    observer.observe(ruler);
    return () => observer.disconnect();
  }, [options]);

  const overflow = options.slice(visibleCount);
  const currentOverflow = overflow.find((option) => option.value === value);

  return (
    <div ref={containerRef} className="relative flex min-w-0 items-center gap-1">
      {visibleCount > 0 && (
        <ToggleGroup
          type="single"
          size="sm"
          spacing={1}
          aria-label="Category"
          className="shrink-0"
          value={value}
          onValueChange={(next) => next && onChange(next)}
        >
          {options.slice(0, visibleCount).map((option) => (
            <ToggleGroupItem key={option.value} value={option.value}>
              {option.label}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
      )}
      {overflow.length > 0 && (
        <DropdownMenu>
          <Button
            type="button"
            size="icon-sm"
            variant={currentOverflow ? "secondary" : "ghost"}
            aria-label={
              currentOverflow
                ? `More categories (selected: ${currentOverflow.label})`
                : "More categories"
            }
            title={currentOverflow?.label ?? "More categories"}
          >
            <Ellipsis />
          </Button>
          <DropdownMenu.Popover placement="bottom end">
            <DropdownMenu.Menu
              aria-label="More categories"
              selectionMode="single"
              selectedKeys={currentOverflow ? [currentOverflow.value] : []}
            >
              {overflow.map((option) => (
                <DropdownMenuItem
                  key={option.value}
                  id={option.value}
                  textValue={option.label}
                  onSelect={() => onChange(option.value)}
                >
                  <span className="mr-auto">{option.label}</span>
                  {option.value === value && <Check className="size-4" aria-hidden="true" />}
                </DropdownMenuItem>
              ))}
            </DropdownMenu.Menu>
          </DropdownMenu.Popover>
        </DropdownMenu>
      )}
      {/* Measure the intrinsic widths of the same controls, including categories in the menu. */}
      <div inert aria-hidden="true" className="invisible absolute inset-0 overflow-hidden">
        <div ref={rulerRef} className="flex w-max items-center gap-1">
          <ToggleGroup type="single" size="sm" spacing={1}>
            {options.map((option) => (
              <ToggleGroupItem key={option.value} value={`measure-${option.value}`}>
                {option.label}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
          <Button type="button" size="icon-sm" variant="ghost">
            <Ellipsis />
          </Button>
        </div>
      </div>
    </div>
  );
}
