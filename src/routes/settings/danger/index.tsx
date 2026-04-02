import { ResetChordsCard } from '#/components/settings/reset-chords-card.tsx';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/danger/')({
  component: SettingsDangerPage,
});

export function SettingsDangerPage() {
  return (
    <div className="space-y-4">
      <ResetChordsCard />
    </div>
  );
}
