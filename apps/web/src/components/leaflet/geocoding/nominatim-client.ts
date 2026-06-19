interface GeocodeResult {
  displayName: string;
  lat: number;
  lng: number;
  bbox?: GeoJSON.BBox;
  type?: string;
  raw: unknown;
}

interface SearchOptions {
  endpoint?: string;
  countryCodes?: string[];
  viewBox?: [number, number, number, number];
  limit?: number;
  signal?: AbortSignal;
}

interface ReverseOptions {
  endpoint?: string;
  signal?: AbortSignal;
}

interface GeocodingProvider {
  search(query: string, opts?: SearchOptions): Promise<GeocodeResult[]>;
  reverse(
    lat: number,
    lng: number,
    opts?: ReverseOptions,
  ): Promise<GeocodeResult | null>;
}

const DEFAULT_ENDPOINT = "https://nominatim.openstreetmap.org";

// Browsers forbid setting User-Agent from fetch; do not include it here.
// Accept-Language is allowed and gives predictable English output.
const HEADERS: Record<string, string> = { "Accept-Language": "en" };

let lastCallAt = 0;
const POLITE_INTERVAL_MS = 1000;

const polite = async () => {
  const now = Date.now();
  const wait = Math.max(0, POLITE_INTERVAL_MS - (now - lastCallAt));
  if (wait > 0) await new Promise((r) => setTimeout(r, wait));
  lastCallAt = Date.now();
};

interface NominatimSearchHit {
  display_name: string;
  lat: string;
  lon: string;
  boundingbox?: [string, string, string, string];
  type?: string;
}

interface NominatimReverseHit {
  display_name: string;
  lat: string;
  lon: string;
}

const nominatimProvider: GeocodingProvider = {
  async search(query, opts = {}) {
    await polite();
    const url = new URL("/search", opts.endpoint ?? DEFAULT_ENDPOINT);
    url.searchParams.set("q", query);
    url.searchParams.set("format", "json");
    url.searchParams.set("limit", String(opts.limit ?? 10));
    if (opts.countryCodes?.length) {
      url.searchParams.set("countrycodes", opts.countryCodes.join(","));
    }
    if (opts.viewBox) {
      url.searchParams.set("viewbox", opts.viewBox.join(","));
    }
    const res = await fetch(url.toString(), {
      headers: HEADERS,
      signal: opts.signal,
    });
    if (!res.ok) throw new Error(`Nominatim search ${res.status}`);
    const arr = (await res.json()) as NominatimSearchHit[];
    return arr.map((r) => ({
      displayName: r.display_name,
      lat: Number(r.lat),
      lng: Number(r.lon),
      // Nominatim boundingbox is [minLat, maxLat, minLon, maxLon].
      // RFC 7946 GeoJSON.BBox is [west, south, east, north] = [minLon, minLat, maxLon, maxLat].
      bbox: r.boundingbox
        ? [
            Number(r.boundingbox[2]),
            Number(r.boundingbox[0]),
            Number(r.boundingbox[3]),
            Number(r.boundingbox[1]),
          ]
        : undefined,
      type: r.type,
      raw: r,
    }));
  },

  async reverse(lat, lng, opts = {}) {
    await polite();
    const url = new URL("/reverse", opts.endpoint ?? DEFAULT_ENDPOINT);
    url.searchParams.set("lat", String(lat));
    url.searchParams.set("lon", String(lng));
    url.searchParams.set("format", "json");
    const res = await fetch(url.toString(), {
      headers: HEADERS,
      signal: opts.signal,
    });
    if (!res.ok) return null;
    const r = (await res.json()) as NominatimReverseHit | null;
    if (!r) return null;
    return {
      displayName: r.display_name,
      lat: Number(r.lat),
      lng: Number(r.lon),
      raw: r,
    };
  },
};

export {
  type GeocodeResult,
  type SearchOptions,
  type ReverseOptions,
  type GeocodingProvider,
  nominatimProvider,
};
