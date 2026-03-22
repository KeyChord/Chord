import { Input } from "#/components/ui/input.tsx";
import { useChorderState } from "#/utils/state.ts";
import { createFileRoute } from "@tanstack/react-router";
import getPrettyKey from "pretty-key";

export const Route = createFileRoute('/chords/')({
  component: Chords,
})

function formatKeys(keys: string[]) {
  return keys.map((key) => getPrettyKey(key)).join(" ");
}

export function Chords() {
  const [state] = useChorderState();
  const inputValue = state.keyBuffer.length > 0
    ? formatKeys(state.keyBuffer)
    : state.activeChord
      ? formatKeys(state.activeChord.keys)
      : "";

  return (
    <div className="size-full bg-transparent p-3">
      <Input
        readOnly
        value={inputValue}
        placeholder="Start typing a chord"
        className="h-11 w-full border-border/70 bg-background/85 font-mono text-base shadow-sm"
      />
    </div>
  );
}
