import * as React from "react";
import { useCallback, type Ref } from "react";
import { Download } from "lucide-react";
import type { Exporter } from "shadmin-core";
import {
  fetchRelatedRecords,
  useDataProvider,
  useGetResourceLabel,
  useNotify,
  useListContext,
  useResourceTranslation,
} from "shadmin-core";
import { Button } from "@/components/ui/button";
import type { UnknownValue } from "@/lib/unknown-types";

/**
 * A button that exports list data to a file.
 *
 * Respects current filters and sort order, with configurable maximum results.
 * Use a custom exporter to customize the result and fetch related records.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/export-button ExportButton documentation}
 *
 * @example
 * import { CreateButton, ExportButton, TopToolbar } from '@/components/admin';
 *
 * const PostListActions = () => (
 *   <>
 *     <FilterButton />
 *     <CreateButton />
 *     <ExportButton />
 *   </>
 * );
 *
 * export const PostList = () => (
 *   <List actions={<PostListActions />}>
 *     ...
 *   </List>
 * );
 */
function ExportButton(props: ExportButtonProps) {
  const {
    maxResults = 1000,
    onClick,
    label: labelProp,
    icon = defaultIcon,
    exporter: customExporter,
    meta,
    className = "cursor-pointer",
    ref,
  } = props;
  const {
    getData,
    total,
    resource,
    exporter: exporterFromContext,
  } = useListContext();
  const getResourceLabel = useGetResourceLabel();
  const label = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.action.export`,
    baseI18nKey: "ra.action.export",
    options: {
      name: getResourceLabel(resource, 1),
    },
    userText: labelProp,
  });
  const exporter = customExporter || exporterFromContext;
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      if (!getData) {
        throw new Error(
          "ListContext.getData must be defined to use ExportButton.",
        );
      }

      getData({ maxResults, meta })
        .then(
          (data) =>
            // biome-ignore lint/complexity/useOptionalChain: `exporter` is a union of two Exporter types; an optional call (`exporter?.(...)`) is not callable across the union, while the `&&` guard preserves callability.
            exporter &&
            exporter(
              data,
              fetchRelatedRecords(dataProvider),
              dataProvider,
              resource,
            ),
        )
        .catch((error) => {
          console.error(error);
          notify("ra.notification.http_error", { type: "error" });
        });
      if (typeof onClick === "function") {
        onClick(event);
      }
    },
    [
      dataProvider,
      exporter,
      getData,
      notify,
      onClick,
      resource,
      maxResults,
      meta,
    ],
  );

  return (
    <Button
      ref={ref}
      variant="outline"
      onClick={handleClick}
      disabled={total === 0}
      className={className}
    >
      {icon}
      {label}
    </Button>
  );
}

const defaultIcon = <Download />;

interface ExportButtonProps {
  className?: string;
  exporter?: Exporter;
  icon?: React.ReactNode;
  label?: string;
  maxResults?: number;
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  resource?: string;
  meta?: UnknownValue;
  ref?: Ref<HTMLButtonElement>;
}

export { ExportButton, type ExportButtonProps };
