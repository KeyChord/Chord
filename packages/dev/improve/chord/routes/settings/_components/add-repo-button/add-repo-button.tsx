import type { FormEvent } from "react";
import { toast } from "@chord/com.npmjs.sonner";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import { useState } from "react";

export function AddRepoButton({ monorepo = false }: { monorepo?: boolean }) {
  const [repoInput, setRepoInput] = useState("");
  const addGitRepoMutation = useMutation({
    mutationFn: monorepo ? taurpc.addGitMonorepo : taurpc.addGitRepo,
    onSuccess: () => setRepoInput(""),
    onError: (error) => {
      toast.error(
        typeof error === "object" && "Message" in error ? String(error.Message) : String(error),
      );
    },
  });

  function handleAddRepo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!repoInput.trim()) {
      toast.error("Enter a GitHub repo like owner/name or https://github.com/owner/name.");
      return;
    }

    addGitRepoMutation.mutate(repoInput);
  }
  return (
    <form className="flex flex-col gap-3 sm:flex-row" onSubmit={handleAddRepo}>
      <Input
        aria-label={monorepo ? "GitHub chord monorepo" : "GitHub chord repo"}
        value={repoInput}
        onChange={(event) => {
          setRepoInput(event.target.value);
        }}
        placeholder="owner/name or https://github.com/owner/name"
        disabled={addGitRepoMutation.isPending}
      />
      <Button type="submit" disabled={addGitRepoMutation.isPending}>
        {addGitRepoMutation.isPending ? "Adding..." : monorepo ? "Add Monorepo" : "Add Repo"}
      </Button>
    </form>
  );
}
