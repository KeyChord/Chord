import { createFileRoute } from '@tanstack/react-router';
import { PlaceholderChordsCard } from '#/components/settings/placeholder-chords-card.tsx';

export const Route = createFileRoute('/settings/configure/')({
  component: SettingsConfigurePage,
});

function SettingsConfigurePage() {
  return (
    <div className="space-y-4">
      <PlaceholderChordsCard />
    </div>
  );
}
