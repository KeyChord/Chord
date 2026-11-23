/** Tauri uses the platform WebView, which identifies the host in its user agent. */
export const desktopPlatform = (() => {
  if (typeof window === "undefined") return "macos";
  const userAgent = window.navigator.userAgent;
  if (/Windows/i.test(userAgent)) return "windows";
  if (/Linux|X11/i.test(userAgent)) return "linux";
  return "macos";
})();
