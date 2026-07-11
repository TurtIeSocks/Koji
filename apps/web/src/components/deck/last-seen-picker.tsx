import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { toLocalInput } from "./use-last-seen";

interface LastSeenPickerProps {
	value: string;
	onChange: (value: string) => void;
	className?: string;
}

/** "Last seen after" datetime filter for the golbat marker previews. Native
 *  `datetime-local` (no picker lib); an empty value = show all. `max` = now, so
 *  a future instant can't be picked (v1's `disableFuture`). */
export function LastSeenPicker({
	value,
	onChange,
	className,
}: LastSeenPickerProps) {
	const max = useMemo(() => toLocalInput(new Date()), []);
	return (
		<label
			className={cn(
				"flex items-center gap-2 rounded-md border bg-background/95 px-2 py-1 text-xs shadow-sm backdrop-blur",
				className,
			)}
		>
			<span className="whitespace-nowrap text-muted-foreground">
				Last seen after
			</span>
			<input
				type="datetime-local"
				aria-label="Last seen after"
				value={value}
				max={max}
				onChange={(e) => onChange(e.target.value)}
				className="bg-transparent outline-none [color-scheme:light_dark]"
			/>
		</label>
	);
}
