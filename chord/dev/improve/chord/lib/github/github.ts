const githubDotGitSuffixRegex = /\.git$/i;

export function getGitHubSlug(url: string) {
	try {
		const parsedUrl = new URL(url);
		if (!parsedUrl.hostname.endsWith('github.com')) {
			return undefined;
		}

		const [owner, name] = parsedUrl.pathname
			.split('/')
			.filter(Boolean)
			.map(segment => segment.replace(githubDotGitSuffixRegex, ''));

		if (!owner || !name) {
			return undefined;
		}

		return `${owner}/${name}`;
	}
	catch {
		return undefined;
	}
}
