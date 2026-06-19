export class OverpassError extends Error {}
export class OverpassRateLimitError extends OverpassError {}
export class OverpassTimeoutError extends OverpassError {}

interface OverpassOptions {
  endpoint?: string;
  timeoutMs?: number;
  signal?: AbortSignal;
}

const DEFAULT_ENDPOINT = "https://overpass-api.de/api/interpreter";

interface OverpassResponse {
  version?: number;
  generator?: string;
  elements: Array<{
    type: "node" | "way" | "relation";
    id: number;
    [k: string]: unknown;
  }>;
}

export async function queryOverpass(
  query: string,
  opts: OverpassOptions = {},
): Promise<OverpassResponse> {
  const endpoint = opts.endpoint ?? DEFAULT_ENDPOINT;
  const controller = new AbortController();
  const timeout = setTimeout(
    () => controller.abort(),
    opts.timeoutMs ?? 30_000,
  );
  try {
    const res = await fetch(endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: `data=${encodeURIComponent(query)}`,
      signal: opts.signal ?? controller.signal,
    });
    if (!res.ok) {
      if (res.status === 429)
        throw new OverpassRateLimitError(`Rate limited: ${res.statusText}`);
      if (res.status === 504)
        throw new OverpassTimeoutError(`Server timeout: ${res.statusText}`);
      throw new OverpassError(
        `Overpass error ${res.status}: ${res.statusText}`,
      );
    }
    return (await res.json()) as OverpassResponse;
  } finally {
    clearTimeout(timeout);
  }
}

export { type OverpassOptions, type OverpassResponse };
