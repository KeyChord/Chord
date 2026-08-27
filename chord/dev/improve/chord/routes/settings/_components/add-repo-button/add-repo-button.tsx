import type { FormEvent } from 'react';
import { toast } from '@chord/com.npmjs.sonner';
import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { Input } from '@chord/dev.improve.chord.components.ui.input';
import { useState } from 'react';

export function AddRepoButton() {
	const [repoInput, setRepoInput] = useState('');
	const addGitRepoMutation = useMutation({
		mutationFn: taurpc.addGitRepo,
	});

	function handleAddRepo(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();

		if (!repoInput.trim()) {
			toast.error('Enter a GitHub repo like owner/name or https://github.com/owner/name.');
			return;
		}

		addGitRepoMutation.mutate(repoInput);
	}
	return (
		<form className="flex flex-col gap-3 sm:flex-row" onSubmit={handleAddRepo}>
			<Input
				value={repoInput}
				onChange={(event) => {
					setRepoInput(event.target.value);
				}}
				placeholder="owner/name or https://github.com/owner/name"
				disabled={addGitRepoMutation.isPending}
			/>
			<Button type="submit" disabled={addGitRepoMutation.isPending}>
				{addGitRepoMutation.isPending ? 'Adding...' : 'Add Repo'}
			</Button>
		</form>
	);
}
