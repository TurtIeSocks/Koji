import type { Ref } from "react";
import { Link } from "react-router";
import { ChevronLeft, ChevronRight, AlertCircle } from "lucide-react";
import {
  type RaRecord,
  type UsePrevNextControllerProps,
  usePrevNextController,
  useTranslate,
} from "shadmin-core";

import { Button, buttonVariants } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

interface PrevNextButtonsProps<RecordType extends RaRecord = RaRecord>
  extends UsePrevNextControllerProps<RecordType> {
  className?: string;
  ref?: Ref<HTMLElement>;
}

/**
 * Renders previous/next record navigation buttons for a Show or Edit view.
 *
 * Uses `usePrevNextController` to fetch the surrounding records and displays
 * the current index out of the total. Must be used within a `RecordContext`,
 * typically inside a `<Show>` or `<Edit>` view.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/prev-next-buttons PrevNextButtons documentation}
 *
 * @example
 * import { Edit, PrevNextButtons } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit actions={<PrevNextButtons />}>
 *     ...
 *   </Edit>
 * );
 */
function PrevNextButtons<RecordType extends RaRecord = RaRecord>(
  props: PrevNextButtonsProps<RecordType>,
) {
  const { className, ref } = props;
  const {
    hasPrev,
    hasNext,
    prevPath,
    nextPath,
    index,
    total,
    error,
    isPending,
  } = usePrevNextController<RecordType>(props);
  const translate = useTranslate();

  if (isPending) {
    return (
      <nav
        className={cn("inline-flex items-center gap-2 min-h-[34px]", className)}
      >
        <Skeleton className="h-1 w-24" />
      </nav>
    );
  }
  if (error) {
    return (
      <AlertCircle
        className="size-4 text-destructive"
        aria-label={translate("ra.notification.http_error", { _: "Error" })}
      />
    );
  }
  if (!hasPrev && !hasNext) {
    return <div className="min-h-[34px]" />;
  }

  return (
    <nav className={cn("inline-flex items-center gap-2", className)} ref={ref}>
      {hasPrev && prevPath ? (
        <Link
          to={prevPath}
          className={buttonVariants({ variant: "ghost", size: "icon" })}
          aria-label={translate("ra.navigation.previous")}
        >
          <ChevronLeft />
        </Link>
      ) : (
        <Button
          variant="ghost"
          size="icon"
          disabled
          aria-label={translate("ra.navigation.previous")}
        >
          <ChevronLeft />
        </Button>
      )}

      {typeof index === "number" && (
        <span className="text-sm tabular-nums">
          {index + 1} / {total}
        </span>
      )}

      {hasNext && nextPath ? (
        <Link
          to={nextPath}
          className={buttonVariants({ variant: "ghost", size: "icon" })}
          aria-label={translate("ra.navigation.next")}
        >
          <ChevronRight />
        </Link>
      ) : (
        <Button
          variant="ghost"
          size="icon"
          disabled
          aria-label={translate("ra.navigation.next")}
        >
          <ChevronRight />
        </Button>
      )}
    </nav>
  );
}

export { PrevNextButtons, type PrevNextButtonsProps };
