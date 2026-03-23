import { Button } from "#/components/ui/button.tsx";
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