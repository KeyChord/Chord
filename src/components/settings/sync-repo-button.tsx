                      import { Check, ExternalLink } from "lucide-react";
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

export function SyncRepoButton({ repo }: { repo: { slug: string } }) {
  const syncGitRepoMutation = useMutation({
    mutationFn: taurpc.syncGitRepo
  })

                      return <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          syncGitRepoMutation.mutate(repo.slug)
                        }}
                        disabled={syncGitRepoMutation.isPending}
                      >
                        {syncGitRepoMutation.isPending ? "Syncing..." : "Sync Latest"}
                      </Button>
                      }