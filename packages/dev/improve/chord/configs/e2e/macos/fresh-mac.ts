import type { Guest } from "#guest";
import { center, type Screen } from "#screen";
import { waitFor } from "#wait";

/**
 * The image disables Gatekeeper, and turning it back on selects "App Store" alone. A new Mac
 * allows "App Store & Known Developers". Only System Settings can select that with SIP on.
 */
export async function allowKnownDevelopers(guest: Guest, screen: Screen) {
  await screen.wake();
  await guest.run(
    'open "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Security"',
  );
  // The menu is the other text on the row, showing the current choice. The window scrolls to this
  // row as it opens, so both come from one look at the screen, and only once the row stops moving.
  let previousRow: number | undefined;
  const menu = await waitFor(
    "the Allow applications from menu",
    async () => {
      const boxes = await screen.read();
      const label = boxes.find((box) => box.text.includes("Allow applications from"));
      if (!label) return undefined;
      const row = center(label).y;
      const settled = previousRow !== undefined && Math.abs(row - previousRow) < 2;
      previousRow = row;
      if (!settled) return undefined;
      return boxes.find(
        (box) => Math.abs(center(box).y - row) < 20 && box.text.includes("App Store"),
      );
    },
    { timeoutMs: 60_000, intervalMs: 1_000 },
  );
  await screen.vnc.click(center(menu).x, center(menu).y);
  await screen.clickText("Known Developers", { timeoutMs: 5_000 });
  await screen.authorize();
  await guest.run('killall "System Settings"');
  const { stdout } = await guest.run("spctl --status --verbose; true");
  if (!stdout.includes("developer id enabled")) {
    throw new Error(`Gatekeeper is not set like a new Mac's: ${stdout.trim()}`);
  }
}
