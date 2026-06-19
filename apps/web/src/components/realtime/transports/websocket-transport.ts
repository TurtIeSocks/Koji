import type {
  RealtimeEvent,
  RealtimeTransport,
  RealtimeTransportError,
  SubscriptionCallback,
  Unsubscribe,
} from "../types";

export interface WebSocketReconnectConfig {
  enabled?: boolean;
  initialDelayMs?: number;
  maxDelayMs?: number;
  maxAttempts?: number;
  jitter?: number;
}

export interface WebSocketTransportConfig {
  url: string;
  protocols?: string | string[];
  getAuthToken?: () => string | Promise<string>;
  authMode?: "query" | "subprotocol";
  reconnect?: WebSocketReconnectConfig;
  idleDisconnectMs?: number;
  heartbeatMs?: number;
  onError?: (error: RealtimeTransportError) => void;
}

type ClientFrame =
  | { op: "subscribe"; topic: string }
  | { op: "unsubscribe"; topic: string }
  | { op: "publish"; topic: string; event: Omit<RealtimeEvent, "topic"> }
  | { op: "ping" };

interface ServerFrame {
  topic?: string;
  type?: string;
  payload?: unknown;
  meta?: Record<string, unknown>;
  op?: "pong";
}

interface PendingPublish {
  topic: string;
  event: Omit<RealtimeEvent, "topic">;
  resolve: () => void;
  reject: (err: unknown) => void;
}

export function webSocketTransport(
  config: WebSocketTransportConfig,
): RealtimeTransport {
  const reconnectCfg: Required<WebSocketReconnectConfig> = {
    enabled: config.reconnect?.enabled ?? true,
    initialDelayMs: config.reconnect?.initialDelayMs ?? 1000,
    maxDelayMs: config.reconnect?.maxDelayMs ?? 30000,
    maxAttempts: config.reconnect?.maxAttempts ?? Infinity,
    jitter: config.reconnect?.jitter ?? 0.3,
  };
  const idleDisconnectMs = config.idleDisconnectMs ?? 30000;
  const heartbeatMs = config.heartbeatMs ?? 30000;

  const subscribers = new Map<string, Set<SubscriptionCallback<unknown>>>();
  const reconnectListeners = new Set<() => void>();
  const pendingPublishes: PendingPublish[] = [];

  let ws: WebSocket | null = null;
  let isFirstOpen = true;
  let reconnectAttempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;
  let pongTimer: ReturnType<typeof setTimeout> | null = null;
  let intentionalClose = false;

  function clearTimers() {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (idleTimer !== null) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
    if (heartbeatTimer !== null) {
      clearTimeout(heartbeatTimer);
      heartbeatTimer = null;
    }
    if (pongTimer !== null) {
      clearTimeout(pongTimer);
      pongTimer = null;
    }
  }

  function rejectPendingPublishes(error: Error) {
    for (const pending of pendingPublishes.splice(0)) {
      pending.reject(error);
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

  function sendFrame(frame: ClientFrame): boolean {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(frame));
      return true;
    }
    return false;
  }

  function scheduleHeartbeat() {
    if (heartbeatMs <= 0) return;
    heartbeatTimer = setTimeout(() => {
      sendFrame({ op: "ping" });
      pongTimer = setTimeout(
        () => {
          // no pong — force close to trigger reconnect
          ws?.close();
        },
        Math.ceil(heartbeatMs / 3),
      );
    }, heartbeatMs);
  }

  async function buildUrl(): Promise<string> {
    if (!config.getAuthToken || config.authMode === "subprotocol") {
      return config.url;
    }
    const token = await config.getAuthToken();
    const sep = config.url.includes("?") ? "&" : "?";
    return `${config.url}${sep}token=${encodeURIComponent(token)}`;
  }

  async function openSocket() {
    let url: string;
    try {
      url = await buildUrl();
    } catch (cause) {
      config.onError?.({ kind: "auth_failed", cause, retrying: false });
      return;
    }

    let protocols: string | string[] | undefined = config.protocols;
    if (config.getAuthToken && config.authMode === "subprotocol") {
      try {
        const token = await config.getAuthToken();
        protocols = Array.isArray(protocols)
          ? [...protocols, token]
          : protocols
            ? [protocols, token]
            : token;
      } catch (cause) {
        config.onError?.({ kind: "auth_failed", cause, retrying: false });
        return;
      }
    }

    const socket = protocols
      ? new WebSocket(url, protocols)
      : new WebSocket(url);
    ws = socket;

    socket.onopen = () => {
      reconnectAttempts = 0;
      if (!isFirstOpen) {
        for (const cb of reconnectListeners) cb();
      }
      isFirstOpen = false;

      // re-subscribe all active topics
      for (const topic of subscribers.keys()) {
        sendFrame({ op: "subscribe", topic });
      }

      // flush pending publishes
      for (const pending of pendingPublishes.splice(0)) {
        const sent = sendFrame({
          op: "publish",
          topic: pending.topic,
          event: pending.event,
        });
        if (sent) {
          pending.resolve();
        } else {
          pending.reject(
            new Error("webSocketTransport: send failed after reconnect"),
          );
        }
      }

      scheduleHeartbeat();
    };

    socket.onmessage = (ev) => {
      // reset heartbeat on any message
      if (pongTimer !== null) {
        clearTimeout(pongTimer);
        pongTimer = null;
      }
      if (heartbeatTimer !== null) {
        clearTimeout(heartbeatTimer);
        heartbeatTimer = null;
      }
      scheduleHeartbeat();

      let frame: ServerFrame;
      try {
        frame = JSON.parse(String(ev.data)) as ServerFrame;
      } catch (cause) {
        config.onError?.({ kind: "parse_failed", cause, retrying: false });
        return;
      }

      if (frame.op === "pong") return;

      const { topic, type, payload, meta } = frame;
      if (!topic || !type) return;

      const set = subscribers.get(topic);
      if (!set) return;

      const full: RealtimeEvent = {
        topic,
        type,
        payload,
        ...(meta ? { meta } : {}),
      };
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

    socket.onclose = (ev) => {
      clearTimers();
      ws = null;
      if (intentionalClose) {
        rejectPendingPublishes(
          new Error("webSocketTransport: connection closed"),
        );
        return;
      }

      const unclean = !ev.wasClean;
      const willRetry =
        unclean &&
        reconnectCfg.enabled &&
        reconnectAttempts < reconnectCfg.maxAttempts;
      if (willRetry) {
        const delay = jitteredDelay(reconnectAttempts++);
        reconnectTimer = setTimeout(() => {
          void openSocket();
        }, delay);
      } else {
        // Reconnect disabled or attempts exhausted — any queued publishes will
        // never be flushed. Reject them so callers can react instead of hanging.
        config.onError?.({ kind: "connect_failed", retrying: false });
        rejectPendingPublishes(
          new Error(
            "webSocketTransport: reconnect exhausted; pending publishes dropped",
          ),
        );
      }
    };

    socket.onerror = () => {
      config.onError?.({
        kind: "connect_failed",
        retrying: reconnectCfg.enabled,
      });
    };
  }

  function startIdleTimer() {
    if (idleDisconnectMs <= 0) return;
    idleTimer = setTimeout(() => {
      if (subscribers.size === 0) {
        intentionalClose = true;
        clearTimers();
        ws?.close();
        ws = null;
        intentionalClose = false;
      }
    }, idleDisconnectMs);
  }

  function ensureConnected() {
    if (
      !ws ||
      ws.readyState === WebSocket.CLOSING ||
      ws.readyState === WebSocket.CLOSED
    ) {
      void openSocket();
    }
    if (idleTimer !== null) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
  }

  return {
    subscribe<P = unknown>(
      topic: string,
      cb: SubscriptionCallback<P>,
    ): Unsubscribe {
      ensureConnected();

      let set = subscribers.get(topic);
      if (!set) {
        set = new Set();
        subscribers.set(topic, set);
        // send subscribe frame if already connected
        sendFrame({ op: "subscribe", topic });
      }
      set.add(cb as SubscriptionCallback<unknown>);

      return () => {
        const s = subscribers.get(topic);
        if (s) {
          s.delete(cb as SubscriptionCallback<unknown>);
          if (s.size === 0) {
            subscribers.delete(topic);
            sendFrame({ op: "unsubscribe", topic });
          }
        }
        if (subscribers.size === 0) startIdleTimer();
      };
    },

    async publish<P = unknown>(
      topic: string,
      event: Omit<RealtimeEvent<P>, "topic">,
    ): Promise<void> {
      ensureConnected();
      const frame: ClientFrame = {
        op: "publish",
        topic,
        event: event as Omit<RealtimeEvent, "topic">,
      };
      if (ws && ws.readyState === WebSocket.OPEN) {
        try {
          ws.send(JSON.stringify(frame));
        } catch (cause) {
          config.onError?.({
            kind: "send_failed",
            topic,
            cause,
            retrying: false,
          });
          throw cause;
        }
        return;
      }
      // queue until open
      return new Promise<void>((resolve, reject) => {
        pendingPublishes.push({
          topic,
          event: event as Omit<RealtimeEvent, "topic">,
          resolve,
          reject,
        });
      });
    },

    connect() {
      intentionalClose = false;
      ensureConnected();
    },

    disconnect() {
      intentionalClose = true;
      clearTimers();
      ws?.close();
      ws = null;
      rejectPendingPublishes(
        new Error(
          "webSocketTransport: disconnected; pending publishes dropped",
        ),
      );
    },

    onReconnect(cb: () => void): Unsubscribe {
      reconnectListeners.add(cb);
      return () => {
        reconnectListeners.delete(cb);
      };
    },
  };
}
