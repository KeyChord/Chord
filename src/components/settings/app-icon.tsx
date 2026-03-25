import type { DesktopAppMetadata } from '../../types/generated.ts';
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from '#/components/ui/tooltip.tsx';

export function AppIcon({
	appMetadata,
	label,
	tooltip,
}: {
	appMetadata?: DesktopAppMetadata
	label: string
	tooltip?: string
}) {
	const fallback = label.trim().charAt(0).toUpperCase() || '?';
	const icon = (
		<div className="flex size-5 shrink-0 items-center justify-center overflow-hidden rounded-md border bg-muted text-[10px] font-medium text-muted-foreground">
			{appMetadata?.iconDataUrl
				? (
						<img src={appMetadata.iconDataUrl} alt="" className="size-full object-contain" />
					)
				: (
						<span aria-hidden="true">{fallback}</span>
					)}
			<span className="sr-only">{label}</span>
		</div>
	);

	if (!tooltip) {
		return icon;
	}

	return (
		<TooltipProvider>
			<Tooltip>
				<TooltipTrigger asChild>{icon}</TooltipTrigger>
				<TooltipContent sideOffset={8}>{tooltip}</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
}
