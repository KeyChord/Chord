/**
 * X11 keysyms, as sent in RFB key events. Virtualization.framework's VNC server turns
 * them into presses on the VM's virtual keyboard, so the guest sees real HID input:
 * `UserKeyMapping` applies and Chord's event tap receives the same events as from a
 * physical keyboard.
 */
export const Keysym = {
  Space: 0x0020,
  Backspace: 0xff08,
  Tab: 0xff09,
  Return: 0xff0d,
  Escape: 0xff1b,
  Left: 0xff51,
  Up: 0xff52,
  Right: 0xff53,
  Down: 0xff54,
  Shift: 0xffe1,
  Control: 0xffe3,
  CapsLock: 0xffe5,
  /** Virtualization.framework maps Alt_L to Command; Cirrus Labs' image templates rely on it. */
  Command: 0xffe9,
} as const;

/** F1 is 0xffbe; the function keys are consecutive up to F35. */
export function functionKey(n: number): number {
  if (!Number.isInteger(n) || n < 1 || n > 35) throw new RangeError(`no function key F${n}`);
  return 0xffbe + n - 1;
}

/** Shifted characters on a US keyboard and the key that produces them. */
const SHIFTED: Record<string, string> = {
  "~": "`",
  "!": "1",
  "@": "2",
  "#": "3",
  $: "4",
  "%": "5",
  "^": "6",
  "&": "7",
  "*": "8",
  "(": "9",
  ")": "0",
  _: "-",
  "+": "=",
  "{": "[",
  "}": "]",
  "|": "\\",
  ":": ";",
  '"': "'",
  "<": ",",
  ">": ".",
  "?": "/",
};

export interface Stroke {
  keysym: number;
  shift: boolean;
}

/**
 * The key that types `char` on a US layout. Shifted characters are sent as their base
 * key with Shift held, so the result does not depend on how the server maps keysyms.
 */
export function strokeFor(char: string): Stroke {
  if (char === "\n") return { keysym: Keysym.Return, shift: false };
  if (char === "\t") return { keysym: Keysym.Tab, shift: false };
  if (char >= "A" && char <= "Z") return { keysym: char.toLowerCase().charCodeAt(0), shift: true };
  const base = SHIFTED[char];
  if (base) return { keysym: base.charCodeAt(0), shift: true };
  const code = char.charCodeAt(0);
  if (char.length === 1 && code >= 0x20 && code <= 0x7e) return { keysym: code, shift: false };
  throw new Error(`cannot type ${JSON.stringify(char)} on a US keyboard`);
}
