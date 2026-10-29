import { Navigate } from "@chord/com.npmjs.tanstack__react-router";

/** Keep saved links to the former My Chords tab working. */
export function SettingsPersonalPage() {
  return <Navigate to="/settings/chords" replace />;
}
