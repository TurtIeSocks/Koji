"use client";

import { useQuery } from "@tanstack/react-query";

import {
  queryOverpass,
  type OverpassOptions,
  type OverpassResponse,
} from "./overpass-client";

const useOverpass = (
  query: string | null,
  opts: OverpassOptions & { enabled?: boolean; staleTime?: number } = {},
) => {
  return useQuery<OverpassResponse>({
    queryKey: ["overpass", query, opts.endpoint],
    queryFn: () => queryOverpass(query!, opts),
    enabled: (opts.enabled ?? true) && query != null && query.length > 0,
    staleTime: opts.staleTime ?? 60 * 60 * 1000,
  });
};

export { useOverpass };
