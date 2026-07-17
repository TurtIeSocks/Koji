import {
  webSocketTransport,
  type RealtimeTransport,
} from "@/components/realtime";

/** Live realtime transport: the server's `/internal/realtime` WebSocket. */
export const createRealtimeTransport = (): RealtimeTransport =>
  webSocketTransport({ url: "/internal/realtime" });
