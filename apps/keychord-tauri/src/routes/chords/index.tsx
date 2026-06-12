import { createFileRoute } from '@tanstack/react-router'
import { ChordsPage } from '@keychord/dev.improve.keychord.routes.chords.index';

export const Route = createFileRoute('/chords/')({
	component: ChordsPage,
});
