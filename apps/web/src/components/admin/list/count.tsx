import type { SortPayload } from "shadmin-core";
import {
  useResourceContext,
  useGetList,
  useTimeout,
  useCreatePath,
} from "shadmin-core";
import { CircleX } from "lucide-react";

import { Link } from "react-router";
import { Spinner } from "@/components/ui/spinner";
import type { UnknownValue } from "@/lib/unknown-types";

/**
 * Fetches and displays the item count for a resource.
 *
 * Uses dataProvider.getList() with minimal pagination to fetch the total count.
 * Displays a loading spinner while fetching and an error icon on failure.
 * By default, uses the current resource from ResourceContext.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/count Count documentation}
 *
 * @example
 * import { Count } from '@/components/admin';
 * // Display the number of records in the current resource
 * <Count />
 *
 * @example
 * // Display the number of posts
 * <Count resource="posts" />
 *
 * @example
 * // Display the number of published posts with a link
 * <Count resource="posts" filter={{ is_published: true }} link />
 */
function Count(props: CountProps) {
  const {
    filter,
    sort,
    link,
    resource: _resourceFromProps,
    timeout = 1000,
    ...rest
  } = props;
  const resource = useResourceContext(props);
  if (!resource) {
    throw new Error(
      "The Count component must be used inside a ResourceContext or must be passed a resource prop.",
    );
  }
  const oneSecondHasPassed = useTimeout(timeout);
  const createPath = useCreatePath();

  const { total, isPending, error } = useGetList(resource, {
    filter,
    sort,
    pagination: { perPage: 1, page: 1 },
  });

  const body = isPending ? (
    oneSecondHasPassed ? (
      <Spinner />
    ) : (
      ""
    )
  ) : error ? (
    <CircleX color="error" />
  ) : (
    total
  );

  return link ? (
    <Link
      to={{
        pathname: createPath({ resource, type: "list" }),
        search: filter ? `filter=${JSON.stringify(filter)}` : undefined,
      }}
      onClick={(e) => e.stopPropagation()}
      {...rest}
    >
      {body}
    </Link>
  ) : (
    <span {...rest}>{body}</span>
  );
}

interface CountProps {
  filter?: UnknownValue;
  sort?: SortPayload;
  link?: boolean;
  resource?: string;
  timeout?: number;
}

export { Count, type CountProps };
