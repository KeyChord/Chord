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
import { useState, type FormEvent } from "react";
import { useAppSettingsState } from "../../utils/state.ts";

export function AddRepoButton() {
  const [repoInput, setRepoInput] = useState("");
   const addGitRepoMutation = useMutation({
    mutationFn: taurpc.addGitRepo
  })

  function handleAddRepo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!repoInput.trim()) {
      toast.error("Enter a GitHub repo like owner/name or https://github.com/owner/name.");
      return;
    }

    addGitRepoMutation.mutate(repoInput)
  }
          return <form className="flex flex-col gap-3 sm:flex-row" onSubmit={handleAddRepo}>
            <Input
              value={repoInput}
              onChange={(event) => {
                setRepoInput(event.target.value);
              }}
              placeholder="owner/name or https://github.com/owner/name"
              disabled={addGitRepoMutation.isPending}
            />
            <Button type="submit" disabled={addGitRepoMutation.isPending}>
              {addGitRepoMutation.isPending ? "Adding..." : "Add Repo"}
            </Button>
          </form>
            }