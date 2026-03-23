import { Button } from "#/components/ui/button.tsx";
import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Collapsible, CollapsibleContent } from "#/components/ui/collapsible.tsx";
import { ChordGroupList } from "#/components/settings/chord-views.tsx";
import { buildChordGroups, getAppLabel } from "#/utils/settings.ts";
import type { SettingsPageData } from "#/utils/use-settings-page.ts";
import { useAppSettingsState } from "../../utils/state.ts";
import { AddRepoButton } from "./add-repo-button.tsx";
import { OpenRepoButton } from "./open-repo-button.tsx";
import { AppsNeedingRelaunchCard } from "./apps-needing-relaunch-card.tsx";
import { useMutation } from "@tanstack/react-query";
import { taurpc } from "../../api/taurpc.ts";

  export function LocalFoldersCard() {
    const pickLocalChordFolderMutation = useMutation({
      mutationFn: taurpc.pickLocalChordFolder
    })

     return <Card size="sm">
        <CardHeader className="flex items-center justify-between gap-3">
          <CardTitle>Local Folders</CardTitle>
          <CardDescription>
            Local folders are loaded in place. Use the tray reload action after editing files to rebuild the JS runtime.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4 pt-0">
          <div className="flex justify-end">
            <Button
              type="button"
              onClick={() => {
                pickLocalChordFolderMutation.mutate()
              }}
              disabled={pickLocalChordFolderMutation.isPending}
            >
              {pickLocalChordFolderMutation.isPending ? "Adding..." : "Add Folder"}
            </Button>
          </div>
        </CardContent>
      </Card>
     }