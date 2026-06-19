import type {
  RealtimeEvent,
  RealtimeTransport,
  SubscriptionCallback,
  Unsubscribe,
} from "../types";

export interface BroadcastChannelTransportConfig {
  channel: string;
}

interface WireMessage {
  topic: string;
  event: Omit<RealtimeEvent, "topic">;
}

export function broadcastChannelTransport(
  config: BroadcastChannelTransportConfig,
): RealtimeTransport {
  let channel: BroadcastChannel | null = new BroadcastChannel(config.channel);
  const subscribers = new Map<string, Set<SubscriptionCallback<unknown>>>();

  const handleMessage = (msg: MessageEvent<WireMessage>) => {
    const { topic, event } = msg.data;
    const set = subscribers.get(topic);
    if (!set) return;
    const full: RealtimeEvent = { topic, ...event };
    for (const cb of set) {
      try {
        cb(full);
      } catch {
        // isolated; no onError hook for this transport in v1
      }
    }
  };

  channel.addEventListener("message", handleMessage);

  return {
    subscribe<P = unknown>(
      topic: string,
      cb: SubscriptionCallback<P>,
    ): Unsubscribe {
      let set = subscribers.get(topic);
      if (!set) {
        set = new Set();
        subscribers.set(topic, set);
      }
      set.add(cb as SubscriptionCallback<unknown>);
      return () => {
        set!.delete(cb as SubscriptionCallback<unknown>);
        if (set!.size === 0) subscribers.delete(topic);
      };
    },

    async publish(topic, event) {
      channel?.postMessage({ topic, event } satisfies WireMessage);
    },

    connect() {
      if (!channel) {
        channel = new BroadcastChannel(config.channel);
        channel.addEventListener("message", handleMessage);
      }
    },

    disconnect() {
      channel?.removeEventListener("message", handleMessage);
      channel?.close();
      channel = null;
    },
  };
}
