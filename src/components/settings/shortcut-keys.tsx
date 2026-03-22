import { Kbd, KbdGroup } from "#/components/ui/kbd.tsx";
import getPrettyKey from "pretty-key";

export function ShortcutKeys({ shortcut }: { shortcut: string }) {
  const chords = shortcut.split(" ").map((chord) => chord.split("+"));

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {chords.map((keys, chordIndex) => (
        <div key={`${shortcut}:${keys.join("+")}:${chordIndex}`} className="flex items-center gap-1.5">
          <KbdGroup>
            {keys.map((key) => (
              <Kbd key={key} className="font-mono text-[11px]">
                {getPrettyKey(key)}
              </Kbd>
            ))}
          </KbdGroup>
          {chordIndex + 1 < chords.length ? (
            <span className="text-xs text-muted-foreground">then</span>
          ) : null}
        </div>
      ))}
    </div>
  );
}
