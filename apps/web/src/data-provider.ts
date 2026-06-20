/* eslint-disable @typescript-eslint/no-explicit-any */
import type { DataProvider, RaRecord } from "ra-core";
import { internalFetch, unwrapResponse, type Meta } from "@/lib/http";

interface ResourceDef {
  seg: string;
  geo: boolean;
}

const RESOURCE_MAP: Record<string, ResourceDef> = {
  geofence: { seg: "geofences", geo: true },
  route: { seg: "routes", geo: true },
  project: { seg: "projects", geo: false },
  property: { seg: "properties", geo: false },
  tileserver: { seg: "tile-servers", geo: false },
  plugins: { seg: "plugins", geo: false },
};

const segFor = (resource: string): string =>
  RESOURCE_MAP[resource]?.seg ?? resource;

const itemPath = (resource: string, id: RaRecord["id"]): string => {
  const seg = segFor(resource);
  if (resource === "plugins") {
    const [kind, ...rest] = `${id}`.split(":");
    return `/${seg}/${kind}/${rest.join(":")}`;
  }
  return `/${seg}/${id}`;
};

/** Map a single GeoJSON Feature → flat record (getOne only; lossless). */
const featureToRecord = (feature: any): any => {
  const props = feature?.properties ?? {};
  const id = props.id ?? feature?.id;
  return {
    ...props,
    id,
    name: props.name ?? `${id}`,
    mode: props.mode ?? "unset",
    geometry: feature?.geometry,
    geo_type: feature?.geometry?.type,
  };
};

const toQuery = (params: any): string => {
  const { page, perPage } = params.pagination;
  const q = new URLSearchParams();
  q.set("page", String(page));
  q.set("per_page", String(perPage));
  q.set("sortBy", params.sort.field);
  q.set("order", params.sort.order);
  for (const [k, v] of Object.entries(params.filter ?? {})) {
    if (v != null) q.set(k, String(v));
  }
  return q.toString();
};

const getListImpl = async (resource: string, params: any): Promise<{ data: any[]; total: number }> => {
  const res = await internalFetch(`/${segFor(resource)}?${toQuery(params)}`);
  const data = unwrapResponse<any[]>(res);
  const meta = (res.json as { meta?: Meta }).meta;
  return { data, total: meta?.total ?? data.length };
};

/** Strip a geofence's `properties` rows to the write wire shape
 *  `{ property_id, value }`, dropping read-only keys (id/name/category/...).
 *  Idempotent; returns data unchanged when there are no properties. */
export const serializeGeofenceWrite = (data: any): any => {
  if (!Array.isArray(data?.properties)) return data;
  return {
    ...data,
    properties: data.properties.map((p: any) => ({
      property_id: p.property_id,
      value: p.value,
    })),
  };
};

export const baseDataProvider: DataProvider = {
  getList: (resource, params) => getListImpl(resource, params) as any,
  getManyReference: (resource, params) => getListImpl(resource, params) as any,

  getOne: async (resource, params) => {
    const res = await internalFetch(itemPath(resource, params.id));
    const data = unwrapResponse<any>(res);
    if (RESOURCE_MAP[resource]?.geo) {
      const feature = data?.type === "FeatureCollection" ? data.features?.[0] : data;
      return { data: featureToRecord(feature) };
    }
    return { data };
  },

  getMany: async (resource, params) => {
    const results = await Promise.allSettled(
      params.ids.map((id: any) =>
        baseDataProvider.getOne(resource, { id }).then((r) => r.data),
      ),
    );
    return {
      data: results
        .filter((r): r is PromiseFulfilledResult<any> => r.status === "fulfilled")
        .map((r) => r.value),
    };
  },

  create: async (resource, params) => {
    const body =
      resource === "geofence" ? serializeGeofenceWrite(params.data) : params.data;
    const res = await internalFetch(`/${segFor(resource)}`, {
      method: "POST",
      body: JSON.stringify(body),
    });
    const data = unwrapResponse<any>(res);
    return { data: { ...data, id: data?.id ?? 0 } } as any;
  },

  update: async (resource, params) => {
    const body =
      resource === "geofence" ? serializeGeofenceWrite(params.data) : params.data;
    const res = await internalFetch(itemPath(resource, params.id), {
      method: "PATCH",
      body: JSON.stringify(body),
    });
    const data = unwrapResponse<any>(res);
    return { data: { ...data, id: data?.id ?? params.id } } as any;
  },

  delete: async (resource, params) => {
    await internalFetch(itemPath(resource, params.id), { method: "DELETE" });
    return { data: { id: params.id } } as any;
  },

  updateMany: async (resource, params) => {
    await Promise.allSettled(
      params.ids.map((id: any) =>
        internalFetch(itemPath(resource, id), {
          method: "PATCH",
          body: JSON.stringify(params.data),
        }),
      ),
    );
    return { data: params.ids };
  },

  deleteMany: async (resource, params) => {
    await Promise.allSettled(
      params.ids.map((id: any) =>
        internalFetch(itemPath(resource, id), { method: "DELETE" }),
      ),
    );
    return { data: params.ids };
  },
};

import {
  realtimeDataProvider,
  webSocketTransport,
  inMemoryLockProvider,
} from "@/components/realtime";

export const dataProvider = realtimeDataProvider(
  baseDataProvider,
  webSocketTransport({ url: "/internal/realtime" }),
  { locks: inMemoryLockProvider() },
);
