import { Check, type LucideIcon, X } from "lucide-react";
import { type RaRecord, useFieldValue, useTranslate } from "shadmin-core";

import type { FieldProps } from "@/lib/field-types";
import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

/**
 * Displays a boolean value as a colored check or close icon.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/boolean-field BooleanField documentation}
 *
 * @example
 * import { Show, SimpleShowLayout, BooleanField } from '@/components/admin';
 *
 * const PostShow = () => (
 *   <Show>
 *     <SimpleShowLayout>
 *       <BooleanField source="is_published" />
 *       <BooleanField source="allow_comments" />
 *     </SimpleShowLayout>
 *   </Show>
 * );
 */
function BooleanField<RecordType extends RaRecord = RaRecord>({
  source,
  record,
  defaultValue,
  className,
  TrueIcon = Check,
  FalseIcon = X,
  valueLabelFalse,
  valueLabelTrue,
  looseValue = false,
  empty = null,
}: BooleanFieldProps<RecordType>) {
  const value = useFieldValue({ source, record, defaultValue });
  const isTruthyValue = value === true || (looseValue && value);
  const baseClassName = "size-5 text-foreground";

  if (looseValue || typeof value === "boolean") {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            {isTruthyValue ? (
              TrueIcon ? (
                <TrueIcon className={cn(baseClassName, className)} />
              ) : (
                <div />
              )
            ) : FalseIcon ? (
              <FalseIcon className={cn(baseClassName, className)} />
            ) : (
              <div />
            )}
          </TooltipTrigger>
          <TooltipContent>
            <RenderLabel
              value={!!value}
              valueLabelFalse={valueLabelFalse}
              valueLabelTrue={valueLabelTrue}
            />
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return <>{empty}</>;
}

function RenderLabel({
  value,
  valueLabelTrue,
  valueLabelFalse,
}: Pick<BooleanFieldProps, "valueLabelFalse" | "valueLabelTrue"> & {
  value: boolean;
}) {
  const translate = useTranslate();

  let label = value ? valueLabelTrue : valueLabelFalse;
  if (!label) {
    label = value ? "ra.boolean.true" : "ra.boolean.false";
  }
  if (typeof label === "string") {
    label = translate(label, { _: label });
  }

  return <p>{label}</p>;
}

interface BooleanFieldProps<RecordType extends RaRecord = RaRecord>
  extends FieldProps<RecordType> {
  defaultValue?: unknown;
  TrueIcon?: LucideIcon | null;
  FalseIcon?: LucideIcon | null;
  valueLabelTrue?: string;
  valueLabelFalse?: string;
  looseValue?: boolean;
}

export { BooleanField, type BooleanFieldProps };
