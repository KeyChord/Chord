import { cn } from "@chord/com.npmjs.utils-cn";
import {
  ArrowBigUp,
  ArrowBigUpDash,
  ArrowDown,
  ArrowDownRight,
  ArrowLeft,
  ArrowRight,
  ArrowRightToLine,
  ArrowUp,
  ArrowUpLeft,
  ChevronUp,
  ChevronsDown,
  ChevronsUp,
  CircleArrowOutUpLeft,
  Command,
  CornerDownLeft,
  Delete,
  Globe,
  Option,
  Space,
  type LucideIcon,
} from "lucide-react";

/**
 * Icons for the keys labelled with a symbol, by the character `pretty-key` gives each key.
 * Only these characters match, because a keycap can also show text the user typed, such as "left".
 */
const KEY_SYMBOLS = new Map<string, { icon: LucideIcon; label: string; mirrored?: boolean }>([
  ["⌘", { icon: Command, label: "Command" }],
  ["⌃", { icon: ChevronUp, label: "Control" }],
  ["⇧", { icon: ArrowBigUp, label: "Shift" }],
  ["⌥", { icon: Option, label: "Option" }],
  ["⇪", { icon: ArrowBigUpDash, label: "Caps Lock" }],
  ["🌐", { icon: Globe, label: "Fn" }],
  ["↩", { icon: CornerDownLeft, label: "Return" }],
  ["⇥", { icon: ArrowRightToLine, label: "Tab" }],
  ["⌫", { icon: Delete, label: "Delete" }],
  ["⌦", { icon: Delete, label: "Forward Delete", mirrored: true }],
  ["⎋", { icon: CircleArrowOutUpLeft, label: "Escape" }],
  ["␣", { icon: Space, label: "Space" }],
  ["↑", { icon: ArrowUp, label: "Up Arrow" }],
  ["↓", { icon: ArrowDown, label: "Down Arrow" }],
  ["←", { icon: ArrowLeft, label: "Left Arrow" }],
  ["→", { icon: ArrowRight, label: "Right Arrow" }],
  ["⇱", { icon: ArrowUpLeft, label: "Home" }],
  ["⇲", { icon: ArrowDownRight, label: "End" }],
  ["⇞", { icon: ChevronsUp, label: "Page Up" }],
  ["⇟", { icon: ChevronsDown, label: "Page Down" }],
]);

// Sized in em so symbols follow the key's text. Lucide pads each icon inside its 24-unit grid, so
// the box is larger than the text for the drawn part to match it, and its strokes are thickened
// to match the stems of Geist Mono.
const SYMBOL_SIZE = "1.1em";
const SYMBOL_STROKE = 2.4;

export function hasKeySymbol(key: string) {
  return KEY_SYMBOLS.has(key);
}

/** What a keycap shows for `label`: the key's icon if it has one, otherwise the label as text. */
export function KeyLabel({ label }: { label: string }) {
  const symbol = KEY_SYMBOLS.get(label);
  if (!symbol) return label;
  const Icon = symbol.icon;
  return (
    <Icon
      size={SYMBOL_SIZE}
      strokeWidth={SYMBOL_STROKE}
      aria-label={symbol.label}
      className={cn("inline-block", symbol.mirrored && "-scale-x-100")}
    />
  );
}
