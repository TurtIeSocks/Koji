import { useMemo } from "react";
import { cn } from "@/lib/utils";
import type { TthFilter } from "@/map/lib/calc-request";
import { toLocalInput } from "./use-last-seen";

const TTHS: TthFilter[] = ["All", "Known", "Unknown"];

interface LastSeenPickerProps {
	value: string;
	onChange: (value: string) => void;
	className?: string;
	/** Spawnpoint confirmed/unconfirmed filter. When both `tth` and `onTthChange`
	 *  are given, a Tth dropdown renders in the SAME frame — the two are related
	 *  golbat filters. Omitted for non-spawnpoint / non-cluster modes. */
	tth?: TthFilter;
	onTthChange?: (tth: TthFilter) => void;
}

/** Golbat marker filters pinned to the map corner. Always shows "Last seen
 *  after" (native `datetime-local`, no picker lib; empty = show all, `max` = now
 *  so no future instant). Optionally shows the spawnpoint Tth dropdown in the
 *  same bordered frame. */
export function LastSeenPicker({
	value,
	onChange,
	className,
	tth,
	onTthChange,
}: LastSeenPickerProps) {
	const max = useMemo(() => toLocalInput(new Date()), []);
	const showTth = tth != null && onTthChange != null;
	return (
		<div
			className={cn(
				"flex flex-col gap-1.5 rounded-md border bg-background/95 px-2 py-1.5 text-xs shadow-sm backdrop-blur",
				className,
			)}
		>
			<label className="flex items-center gap-2">
				<span className="whitespace-nowrap text-muted-foreground">
					Last seen after
				</span>
				<input
					type="datetime-local"
					aria-label="Last seen after"
					value={value}
					max={max}
					onChange={(e) => onChange(e.target.value)}
					className="min-w-0 flex-1 bg-transparent outline-none [color-scheme:light_dark]"
				/>
			</label>
			{showTth && (
				<label className="flex items-center justify-between gap-2">
					<span className="whitespace-nowrap text-muted-foreground">Tth</span>
					<select
						aria-label="Tth"
						value={tth}
						onChange={(e) => onTthChange(e.target.value as TthFilter)}
						className="bg-transparent outline-none [color-scheme:light_dark]"
					>
						{TTHS.map((t) => (
							<option key={t} value={t}>
								{t}
							</option>
						))}
					</select>
				</label>
			)}
		</div>
	);
}
