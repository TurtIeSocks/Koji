import { INTERNAL_BASE } from "@/lib/constants";

export interface Meta {
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
  has_next: boolean;
  has_prev: boolean;
}

export interface Envelope<T> {
  status: "ok" | "error";
  data?: T;
  meta?: Meta;
  error?: unknown;
}

/** Error carrying the HTTP status so ra-core's `authProvider.checkError` can
 *  react to 401/403 (redirect to login) instead of the app crashing. */
export class HttpError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "HttpError";
  }
}

export function unwrap<T>(json: Envelope<T> | null | undefined): T {
  if (json == null) {
    throw new Error("internal API returned an empty body");
  }
  if (json.status === "error") {
    throw new Error(
      `internal API error: ${JSON.stringify(json.error ?? "unknown")}`,
    );
  }
  return json.data as T;
}

/** Unwrap an `internalFetch` result: throw `HttpError` (with status) on a
 *  non-2xx response or empty body, else return the envelope's `data`. Use this
 *  in the dataProvider so a 401 becomes an auth error, not a `null.status` crash. */
export function unwrapResponse<T>(res: {
  status: number;
  json: unknown;
}): T {
  if (res.status < 200 || res.status >= 300) {
    throw new HttpError(res.status, `internal API responded ${res.status}`);
  }
  return unwrap(res.json as Envelope<T>);
}

export async function internalFetch(
  path: string,
  init?: RequestInit,
): Promise<{ status: number; json: unknown }> {
  const res = await fetch(`${INTERNAL_BASE}${path}`, {
    credentials: "include",
    ...init,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      ...(init?.headers ?? {}),
    },
  });
  const text = await res.text();
  const json = text ? JSON.parse(text) : null;
  return { status: res.status, json };
}
