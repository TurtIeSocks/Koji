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

export function unwrap<T>(json: Envelope<T>): T {
  if (json.status === "error") {
    throw new Error(
      `internal API error: ${JSON.stringify(json.error ?? "unknown")}`,
    );
  }
  return json.data as T;
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
