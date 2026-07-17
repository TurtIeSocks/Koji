// The demo world's realtime transport — a purely in-memory topic bus. There's
// no server (or WebSocket/SSE) in the static demo, so "realtime" is just a
// local publish/subscribe over a `Map<topic, Set<callback>>`. The calc facade
// (`calc/facade.ts`) publishes `jobs/{id}` status/progress events through this
// SAME singleton instance, and `useCalc` (via `useSubscribe` → the realtime
// data provider → this transport) receives them — that's the whole loop that
// makes an in-browser calc report progress and resolve its result without any
// polling.
//
// `createRealtimeTransport = () => demoBus` is wired into `index.demo.ts`; the
// app's `data-provider.ts` calls it once at module load and wraps the bus in
// `realtimeDataProvider`, so every `useSubscribe` shares this one instance.

import type {
  RealtimeConnectionStatus,
  RealtimeEvent,
  RealtimeTransport,
  SubscriptionCallback,
  Unsubscribe,
} from "@/components/realtime";

/** The demo bus is a `RealtimeTransport` plus a synchronous `emit` — `publish`
 *  schedules `emit` on a microtask (matching how a networked transport delivers
 *  asynchronously), while `emit` dispatches inline for callers that want it. */
export type DemoBus = RealtimeTransport & {
  emit<P = unknown>(topic: string, event: Omit<RealtimeEvent<P>, "topic">): void;
};

function createDemoBus(): DemoBus {
  const subscribers = new Map<string, Set<SubscriptionCallback<unknown>>>();

  function subscribe<P = unknown>(topic: string, cb: SubscriptionCallback<P>): Unsubscribe {
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
        if (s.size === 0) subscribers.delete(topic);
      }
    };
  }

  function emit<P = unknown>(topic: string, event: Omit<RealtimeEvent<P>, "topic">): void {
    const set = subscribers.get(topic);
    if (!set) return;
    const full: RealtimeEvent<P> = { ...event, topic };
    // Snapshot so an unsubscribe/subscribe inside a handler can't perturb this
    // dispatch pass.
    for (const cb of [...set]) {
      try {
        (cb as SubscriptionCallback<P>)(full);
      } catch (err) {
        // A subscriber throwing must never break the bus or sibling handlers.
        console.error("demoBus subscriber threw", err);
      }
    }
  }

  function publish<P = unknown>(topic: string, event: Omit<RealtimeEvent<P>, "topic">): Promise<void> {
    // Deliver on a microtask so `publish` resolves immediately and delivery is
    // observably asynchronous (a subscriber added synchronously after publish
    // would still be missed — the same ordering a real transport has).
    queueMicrotask(() => emit(topic, event));
    return Promise.resolve();
  }

  function onStatusChange(cb: (status: RealtimeConnectionStatus) => void): Unsubscribe {
    // The in-memory bus is always up — report "connected" immediately and never
    // change. Return a no-op unsubscribe.
    cb("connected");
    return () => {};
  }

  return { subscribe, publish, emit, onStatusChange };
}

/** The one shared in-memory realtime bus for the demo. Both the transport
 *  (via `createRealtimeTransport`) and the calc facade import this instance. */
export const demoBus: DemoBus = createDemoBus();

/** `ApiSurface.createRealtimeTransport` for the demo: hand back the singleton
 *  bus (never a fresh instance — the facade publishes to `demoBus` directly, so
 *  the transport the UI subscribes through must be the very same object). */
export const createRealtimeTransport = (): RealtimeTransport => demoBus;
