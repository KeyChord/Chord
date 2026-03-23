import { Button } from "#/components/ui/button.tsx";
import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Checkbox } from "#/components/ui/checkbox.tsx";
import { Collapsible, CollapsibleContent } from "#/components/ui/collapsible.tsx";
import { Input } from "#/components/ui/input.tsx";
import { Label } from "#/components/ui/label.tsx";
import { AppIcon } from "#/components/settings/app-icon.tsx";
import { ChordGroupList } from "#/components/settings/chord-views.tsx";
import { buildChordGroups, getAppLabel } from "#/utils/settings.ts";
import type { SettingsPageData } from "#/utils/use-settings-page.ts";
import { useMutation } from "@tanstack/react-query";
import { taurpc } from "../../api/taurpc.ts";
import { toast } from 'sonner'
import { useEffect, useState, type FormEvent } from "react";
import { useAppSettingsState } from "../../utils/state.ts";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AddRepoButton } from "./add-repo-button.tsx";
import { OpenRepoButton } from "./open-repo-button.tsx";
import { RelaunchAppButton } from "./relaunch-app-button.tsx";
import { useAppMetadataQuery } from "../../utils/app.ts";

export function AppsNeedingRelaunchCard() {
  const { bundleIdsNeedingRelaunch } = useAppSettingsState()

    return <Card size="sm">
      <CardHeader  className="flex items-center justify-between gap-3">
        <CardTitle>Apps Needing Relaunch</CardTitle>
        <CardDescription>
          Scripts can flag apps that should be restarted after they change app state.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 pt-0">
        {bundleIdsNeedingRelaunch.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No apps are currently marked as needing a relaunch.
          </p>
        ) : (
          bundleIdsNeedingRelaunch.map((bundleId) => {
            return <AppNeedingRelaunchRow key={bundleId} app={ { bundleId }} />
          })
        )}
      </CardContent>
    </Card>
}

function AppNeedingRelaunchRow({ app }: { app: { bundleId: string } }) {
  const { data } = useAppMetadataQuery(app.bundleId)
  // TODO
  const appLabel = app.bundleId

              return (
                <div
                  className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2"
                >
                  <div className="flex min-w-0 items-center gap-2">
                    <AppIcon appMetadata={data} label={appLabel} />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium">{appLabel}</p>
                        <Badge variant="secondary">Needs relaunch</Badge>
                      </div>
                    </div>
                  </div>

                  <RelaunchAppButton app={app} />
                </div>
)
}