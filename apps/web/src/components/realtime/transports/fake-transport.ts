import type {
  RealtimeEvent,
  RealtimeTransport,
  RealtimeTransportError,
  SubscriptionCallback,
  Unsubscribe,
} from "../types";

export interface FakeTransportConfig {
  delayMs?: number;
  onError?: (error: RealtimeTransportError) => void;
}

export interface FakeTransport extends RealtimeTransport {
  simulateReconnect(): void;
  readonly publishedEvents: ReadonlyArray<{
    topic: string;
    event: Omit<RealtimeEvent, "topic">;
  }>;
}

export function fakeTransport(config?: FakeTransportConfig): FakeTransport {
  const subscribers = new Map<string, Set<SubscriptionCallback<unknown>>>();
  const reconnectListeners = new Set<() => void>();
  const events: Array<{ topic: string; event: Omit<RealtimeEvent, "topic"> }> =
    [];

  function subscribe<P = unknown>(
    topic: string,
    cb: SubscriptionCallback<P>,
  ): Unsubscribe {
    let set = subscribers.get(topic);
    if (!set) {
      set = new Set<SubscriptionCallback<unknown>>();
      subscribers.set(topic, set);
    }
    set.add(cb as SubscriptionCallback<unknown>);

    return () => {
      const s = subscribers.get(topic);
      if (s) {
        s.delete(cb as SubscriptionCallback<unknown>);
        if (s.size === 0) {
          subscribers.delete(topic);
        }
      }
    };
  }

  async function publish<P = unknown>(
    topic: string,
    event: Omit<RealtimeEvent<P>, "topic">,
  ): Promise<void> {
    events.push({ topic, event: event as Omit<RealtimeEvent, "topic"> });

    if (config?.delayMs) {
      await new Promise<void>((resolve) => setTimeout(resolve, config.delayMs));
    }

    const set = subscribers.get(topic);
    if (!set) return;

    const full: RealtimeEvent<P> = { ...event, topic };

    for (const cb of set) {
      try {
        (cb as SubscriptionCallback<P>)(full);
      } catch (cause) {
        config?.onError?.({
          kind: "handler_threw",
          topic,
          cause,
          retrying: false,
        });
      }
    }
  }

  function onReconnect(cb: () => void): Unsubscribe {
    reconnectListeners.add(cb);
    return () => {
      reconnectListeners.delete(cb);
    };
  }

  function simulateReconnect(): void {
    for (const cb of reconnectListeners) {
      cb();
    }
  }

  return {
    subscribe,
    publish,
    onReconnect,
    simulateReconnect,
    get publishedEvents(): ReadonlyArray<{
      topic: string;
      event: Omit<RealtimeEvent, "topic">;
    }> {
      return events;
    },
  };
}
