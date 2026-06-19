import type { DataProvider } from "shadmin-core";
import type { RealtimeDataProvider } from "./types";
import { recordTopic, resourceTopic } from "./topics";

export function addEventsForMutations<R extends string = string>(
  baseDataProvider: DataProvider<R> | RealtimeDataProvider<R>,
  publisher: Pick<RealtimeDataProvider<R>, "publish">,
): DataProvider<R> {
  return {
    ...baseDataProvider,

    async create(resource, params) {
      const result = await baseDataProvider.create(resource, params);
      await publisher.publish(resourceTopic(resource), {
        type: "created",
        payload: { ids: [result.data.id] },
      });
      return result;
    },

    async update(resource, params) {
      const result = await baseDataProvider.update(resource, params);
      await publisher.publish(recordTopic(resource, params.id), {
        type: "updated",
        payload: { id: params.id, data: result.data },
      });
      await publisher.publish(resourceTopic(resource), {
        type: "updated",
        payload: { ids: [params.id] },
      });
      return result;
    },

    async updateMany(resource, params) {
      const result = await baseDataProvider.updateMany(resource, params);
      await publisher.publish(resourceTopic(resource), {
        type: "updated",
        payload: { ids: result.data ?? params.ids },
      });
      return result;
    },

    async delete(resource, params) {
      const result = await baseDataProvider.delete(resource, params);
      await publisher.publish(recordTopic(resource, params.id), {
        type: "deleted",
        payload: { id: params.id, previousData: params.previousData },
      });
      await publisher.publish(resourceTopic(resource), {
        type: "deleted",
        payload: { ids: [params.id] },
      });
      return result;
    },

    async deleteMany(resource, params) {
      const result = await baseDataProvider.deleteMany(resource, params);
      await publisher.publish(resourceTopic(resource), {
        type: "deleted",
        payload: { ids: result.data ?? params.ids },
      });
      return result;
    },
  };
}
