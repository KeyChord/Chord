import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import { StateQueries, useSettingsState } from "@chord/dev.improve.chord.lib.state";
import { useEffect } from "react";

export function CommandLineCard() {
  return (
    <StateQueries queries={[useSettingsState()]}>
      {(settingsData) => <CommandLineCardContent settingsData={settingsData} />}
    </StateQueries>
  );
}

function CommandLineCardContent({
  settingsData,
}: {
  settingsData: NonNullable<ReturnType<typeof useSettingsState>["data"]>;
}) {
  const { cliCommand, isCliInstalled } = settingsData;
  const install = useMutation({ mutationFn: taurpc.installCli });
  const refresh = useMutation({ mutationFn: taurpc.refreshCliInstallation });
  const { mutate: refreshStatus } = refresh;
  useEffect(() => {
    refreshStatus();
    const onFocus = () => refreshStatus();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refreshStatus]);
  const error = install.error ?? refresh.error;

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Command Line</CardTitle>
        <CardDescription>
          Add <code>{cliCommand}</code> to your PATH to run chords, shell commands, and scripts from
          your terminal.
        </CardDescription>
      </CardHeader>
      <CardContent className="preference-group">
        <div className="preference-row">
          <div className="min-w-0 flex-1">
            <p className="preference-label">Terminal command</p>
            <p className="preference-description">
              Run chords and scripts with <code>{cliCommand}</code>.
            </p>
          </div>
          <Button
            variant="outline"
            disabled={isCliInstalled || install.isPending || refresh.isPending}
            onClick={() => install.mutate()}
          >
            {install.isPending ? "Installing…" : isCliInstalled ? "Installed" : "Install command…"}
          </Button>
        </div>
        <div className="preference-note">
          <p>
            Installs <code>/usr/local/bin/{cliCommand}</code>. macOS may ask for your administrator
            password.
          </p>
          {isCliInstalled && (
            <p role="status" className="mt-1">
              Run <code>{cliCommand} --help</code> to get started. Make sure{" "}
              <code>/usr/local/bin</code> is in your shell’s PATH.
            </p>
          )}
          {error && (
            <p role="alert" className="mt-2 text-destructive">
              {errorMessage(error)}
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

function errorMessage(error: unknown): string {
  if (
    error &&
    typeof error === "object" &&
    "Message" in error &&
    typeof error.Message === "string"
  ) {
    return error.Message;
  }
  return error instanceof Error ? error.message : String(error);
}
