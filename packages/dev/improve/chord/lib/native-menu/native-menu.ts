import { Menu, MenuItem, PredefinedMenuItem } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";

export type NativeMenuEntry =
  | { label: string; onSelect: () => void; disabled?: boolean }
  | "separator";

/**
 * Opens a macOS menu below `anchor` (or at the pointer when there is none). The menu is the
 * system's own, so it looks, animates and handles the keyboard like every other Mac menu.
 */
export async function showNativeMenu(entries: NativeMenuEntry[], anchor?: Element | null) {
  const items = await Promise.all(
    entries.map((entry) =>
      entry === "separator"
        ? PredefinedMenuItem.new({ item: "Separator" })
        : MenuItem.new({
            text: entry.label,
            enabled: !entry.disabled,
            action: entry.onSelect,
          }),
    ),
  );
  const menu = await Menu.new({ items });
  if (anchor) {
    const { left, bottom } = anchor.getBoundingClientRect();
    await menu.popup(new LogicalPosition(left, bottom + 4));
  } else {
    await menu.popup();
  }
}
