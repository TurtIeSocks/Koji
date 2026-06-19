import type { Identifier } from "shadmin-core";

export const resourceTopic = (resource: string): string =>
  `resource/${resource}`;

export const recordTopic = (resource: string, id: Identifier): string =>
  `resource/${resource}/${id}`;

export const lockResourceTopic = (resource: string): string =>
  `lock/${resource}`;

export const lockTopic = (resource: string, id: Identifier): string =>
  `lock/${resource}/${id}`;
