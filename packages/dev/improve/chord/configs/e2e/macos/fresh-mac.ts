import type { Guest } from "#guest";
import { center, type Screen } from "#screen";

/**
 * The image disables Gatekeeper, and turning it back on selects "App Store" alone. A new Mac
 * allows "App Store & Known Developers". Only System Settings can select that with SIP on.
 */
export async function allowKnownDevelopers(guest: Guest, screen: Screen) {
  await screen.wake();
  await guest.run(
    'open "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Security"',
  );
  const label = center(await screen.waitForText("Allow applications from"));
  // The menu is the other text on the row, showing the current choice.
  const menu = (await screen.read()).find(
    (box) => Math.abs(center(box).y - label.y) < 20 && box.text.includes("App Store"),
  );
  if (!menu) throw new Error("cannot find the Allow applications from menu");
  await screen.vnc.click(center(menu).x, center(menu).y);
  await screen.clickText("Known Developers", { timeoutMs: 5_000 });
  await screen.authorize();
  await guest.run('killall "System Settings"');
  const { stdout } = await guest.run("spctl --status --verbose; true");
  if (!stdout.includes("developer id enabled")) {
    throw new Error(`Gatekeeper is not set like a new Mac's: ${stdout.trim()}`);
  }
}
