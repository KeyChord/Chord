import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  DropdownMenu,
  DropdownMenuHeader,
  DropdownMenuSeparator,
} from "@chord/dev.improve.chord.components.ui.dropdown-menu";
import { ListFilter } from "lucide-react";
import type { Key } from "react";

export type InstallFilter = "all" | "installed" | "not-installed";

const installOptions: { value: InstallFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "installed", label: "Installed" },
  { value: "not-installed", label: "Not Installed" },
];

type Option = { value: string; label: string };

/** The Filter button beside the search field: install status and category, one choice each. */
export function FilterMenu({
  install,
  onInstallChange,
  categories,
  category,
  onCategoryChange,
}: {
  install: InstallFilter;
  onInstallChange: (value: InstallFilter) => void;
  /** Every category, starting with All. */
  categories: Option[];
  category: string;
  onCategoryChange: (value: string) => void;
}) {
  const activeCount = Number(install !== "all") + Number(category !== "all");

  return (
    <DropdownMenu>
      <Button
        type="button"
        variant={activeCount > 0 ? "secondary" : "outline"}
        className={activeCount > 0 ? "shrink-0" : "shrink-0 bg-card"}
        aria-label={activeCount > 0 ? `Filter (${activeCount} active)` : "Filter"}
      >
        <ListFilter />
        Filter
        {activeCount > 0 ? (
          <span className="min-w-4 rounded-full bg-foreground px-1 text-center text-[11px] font-semibold text-background tabular-nums">
            {activeCount}
          </span>
        ) : null}
      </Button>
      <DropdownMenu.Popover placement="bottom end" className="min-w-52">
        <DropdownMenu.Menu aria-label="Filter chord repos">
          <FilterSection
            title="Status"
            options={installOptions}
            value={install}
            onChange={(value) => onInstallChange(value as InstallFilter)}
          />
          <DropdownMenuSeparator />
          <FilterSection
            title="Category"
            options={categories}
            value={category}
            onChange={onCategoryChange}
          />
        </DropdownMenu.Menu>
      </DropdownMenu.Popover>
    </DropdownMenu>
  );
}

function FilterSection({
  title,
  options,
  value,
  onChange,
}: {
  title: string;
  options: Option[];
  value: string;
  onChange: (value: string) => void;
}) {
  // Item ids are unique across the whole menu, and both sections have an "all" option.
  const prefix = `${title}:`;
  return (
    <DropdownMenu.Section
      aria-label={title}
      selectionMode="single"
      disallowEmptySelection
      selectedKeys={[prefix + value]}
      onSelectionChange={(keys) => {
        const [next] = keys === "all" ? [] : (keys as Set<Key>);
        if (next !== undefined) {
          onChange(String(next).slice(prefix.length));
        }
      }}
    >
      <DropdownMenuHeader>{title}</DropdownMenuHeader>
      {options.map((option) => (
        <DropdownMenu.Item key={option.value} id={prefix + option.value} textValue={option.label}>
          <DropdownMenu.ItemIndicator />
          {option.label}
        </DropdownMenu.Item>
      ))}
    </DropdownMenu.Section>
  );
}
