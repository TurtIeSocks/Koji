import type {
  RealtimeEvent,
  RealtimeTransport,
  RealtimeTransportError,
  SubscriptionCallback,
  Unsubscribe,
} from "../types";

export interface SSEReconnectConfig {
  enabled?: boolean;
  initialDelayMs?: number;
  maxDelayMs?: number;
  jitter?: number;
}

export interface SSETransportConfig {
  url: string;
  publishUrl?: string;
  getAuthToken?: () => string | Promise<string>;
  withCredentials?: boolean;
  reconnect?: SSEReconnectConfig;
  idleDisconnectMs?: number;
  topicFilterParam?: string;
  onError?: (error: RealtimeTransportError) => void;
}

export function sseTransport(config: SSETransportConfig): RealtimeTransport {
  const reconnectCfg: Required<SSEReconnectConfig> = {
    enabled: config.reconnect?.enabled ?? true,
    initialDelayMs: config.reconnect?.initialDelayMs ?? 1000,
    maxDelayMs: config.reconnect?.maxDelayMs ?? 30000,
    jitter: config.reconnect?.jitter ?? 0.3,
  };
  const idleDisconnectMs = config.idleDisconnectMs ?? 30000;
  const topicFilterParam = config.topicFilterParam ?? "topics";

  const subscribers = new Map<string, Set<SubscriptionCallback<unknown>>>();
  const reconnectListeners = new Set<() => void>();

  let source: EventSource | null = null;
  let isFirstOpen = true;
  let reconnectAttempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;
  // Track bound event handlers so we can remove them on close
  const topicHandlers = new Map<string, (ev: MessageEvent) => void>();

  function clearTimers() {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (idleTimer !== null) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
  }

  function jitteredDelay(attempt: number): number {
    const base = Math.min(
      reconnectCfg.initialDelayMs * 2 ** attempt,
      reconnectCfg.maxDelayMs,
    );
    const j = base * reconnectCfg.jitter;
    return base + (Math.random() * 2 - 1) * j;
  }

  async function buildUrl(): Promise<string> {
    const topics = Array.from(subscribers.keys());
    let url = config.url;
    if (topics.length > 0) {
      const sep = url.includes("?") ? "&" : "?";
      url += `${sep}${topicFilterParam}=${encodeURIComponent(topics.join(","))}`;
    }
    if (config.getAuthToken) {
      try {
        const token = await config.getAuthToken();
        const sep2 = url.includes("?") ? "&" : "?";
        url += `${sep2}token=${encodeURIComponent(token)}`;
      } catch (cause) {
        config.onError?.({ kind: "auth_failed", cause, retrying: false });
      }
    }
    return url;
  }

  function attachTopicHandler(src: EventSource, topic: string) {
    const handler = (ev: MessageEvent) => {
      let parsed: {
        type?: string;
        payload?: unknown;
        meta?: Record<string, unknown>;
      };
      try {
        parsed = JSON.parse(String(ev.data)) as typeof parsed;
      } catch (cause) {
        config.onError?.({
          kind: "parse_failed",
          topic,
          cause,
          retrying: false,
        });
        return;
      }
      const { type, payload, meta } = parsed;
      if (!type) return;
      const full: RealtimeEvent = {
        topic,
        type,
        payload,
        ...(meta ? { meta } : {}),
      };
      const set = subscribers.get(topic);
      if (!set) return;
      for (const cb of set) {
        try {
          cb(full);
        } catch (cause) {
          config.onError?.({
            kind: "handler_threw",
            topic,
            cause,
            retrying: false,
          });
        }
      }
    };
    topicHandlers.set(topic, handler);
    src.addEventListener(topic, handler);
  }

  async function openSource() {
    // close any existing source without triggering reconnect
    if (source) {
      source.onopen = null;
      source.onerror = null;
      source.close();
      source = null;
      topicHandlers.clear();
    }

    let url: string;
    try {
      url = await buildUrl();
    } catch (cause) {
      config.onError?.({ kind: "connect_failed", cause, retrying: false });
      return;
    }

    const es = new EventSource(url, {
      withCredentials: config.withCredentials ?? false,
    });
    source = es;

    es.onopen = () => {
      reconnectAttempts = 0;
      if (!isFirstOpen) {
        for (const cb of reconnectListeners) cb();
      }
      isFirstOpen = false;
    };

    es.onerror = () => {
      config.onError?.({
        kind: "connect_failed",
        retrying: reconnectCfg.enabled,
      });
      es.onopen = null;
      es.onerror = null;
      es.close();
      if (source === es) source = null;
      topicHandlers.clear();

      if (reconnectCfg.enabled && reconnectAttempts < Infinity) {
        const delay = jitteredDelay(reconnectAttempts++);
        reconnectTimer = setTimeout(() => {
          void openSource();
        }, delay);
      }
    };

    // attach handlers for all currently subscribed topics
    for (const topic of subscribers.keys()) {
      attachTopicHandler(es, topic);
    }
  }

  function startIdleTimer() {
    if (idleDisconnectMs <= 0) return;
    idleTimer = setTimeout(() => {
      if (subscribers.size === 0) {
        clearTimers();
        if (source) {
          source.onopen = null;
          source.onerror = null;
          source.close();
          source = null;
          topicHandlers.clear();
        }
      }
    }, idleDisconnectMs);
  }

  function ensureConnected() {
    if (idleTimer !== null) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
    if (!source || source.readyState === EventSource.CLOSED) {
      void openSource();
    }
  }

  return {
    subscribe<P = unknown>(
      topic: string,
      cb: SubscriptionCallback<P>,
    ): Unsubscribe {
      const isNewTopic = !subscribers.has(topic);

      let set = subscribers.get(topic);
      if (!set) {
        set = new Set();
        subscribers.set(topic, set);
      }
      set.add(cb as SubscriptionCallback<unknown>);

      if (isNewTopic && source && source.readyState !== EventSource.CLOSED) {
        // New topic — must re-open to include it in the filter param
        void openSource();
      } else {
        ensureConnected();
      }

      return () => {
        const s = subscribers.get(topic);
        if (s) {
          s.delete(cb as SubscriptionCallback<unknown>);
          if (s.size === 0) {
            subscribers.delete(topic);
            // Remove the handler from the current source
            const handler = topicHandlers.get(topic);
            if (handler && source) source.removeEventListener(topic, handler);
            topicHandlers.delete(topic);
          }
        }
        if (subscribers.size === 0) startIdleTimer();
      };
    },

    async publish<P = unknown>(
      topic: string,
      event: Omit<RealtimeEvent<P>, "topic">,
    ): Promise<void> {
      if (!config.publishUrl) {
        throw new Error("sseTransport: publish requires publishUrl in config");
      }
      let res: Response;
      try {
        res = await fetch(config.publishUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ topic, event }),
        });
      } catch (cause) {
        config.onError?.({
          kind: "send_failed",
          topic,
          cause,
          retrying: false,
        });
        throw cause;
      }
      if (!res.ok) {
        const error = new Error(
          `sseTransport: publish failed with status ${res.status}`,
        );
        config.onError?.({
          kind: "send_failed",
          topic,
          cause: error,
          retrying: false,
        });
        throw error;
      }
    },

    connect() {
      ensureConnected();
    },

    disconnect() {
      clearTimers();
      if (source) {
        source.onopen = null;
        source.onerror = null;
        source.close();
        source = null;
        topicHandlers.clear();
      }
    },

    onReconnect(cb: () => void): Unsubscribe {
      reconnectListeners.add(cb);
      return () => {
        reconnectListeners.delete(cb);
      };
    },
  };
}
