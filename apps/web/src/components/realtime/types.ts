import type { DataProvider, Identifier } from "shadmin-core";

export type RealtimeEventType =
  | "created"
  | "updated"
  | "deleted"
  | "locked"
  | "unlocked"
  | (string & {});

export interface RealtimeEvent<P = unknown> {
  topic: string;
  type: RealtimeEventType;
  payload: P;
  meta?: Record<string, unknown>;
}

export type SubscriptionCallback<P = unknown> = (
  event: RealtimeEvent<P>,
) => void;
export type Unsubscribe = () => void;

export interface Lock {
  resource: string;
  recordId: Identifier;
  identity: Identifier;
  createdAt: string;
}

export interface LockParams {
  id: Identifier;
  identity: Identifier;
  meta?: Record<string, unknown>;
}

export interface UnlockParams {
  id: Identifier;
  identity: Identifier;
  meta?: Record<string, unknown>;
}

export interface GetLockParams {
  id: Identifier;
  meta?: Record<string, unknown>;
}

export interface GetLocksParams {
  meta?: Record<string, unknown>;
}

export class LockConflictError extends Error {
  readonly kind = "LockConflictError" as const;
  constructor(public readonly existingLock: Lock) {
    super(
      `Resource ${existingLock.resource}/${String(existingLock.recordId)} is locked by ${String(existingLock.identity)}`,
    );
  }
}

export type RealtimeConnectionStatus =
  | "connected"
  | "reconnecting"
  | "disconnected"
  | "idle";

export type RealtimeTransportErrorKind =
  | "connect_failed"
  | "auth_failed"
  | "send_failed"
  | "parse_failed"
  | "handler_threw";

export interface RealtimeTransportError {
  kind: RealtimeTransportErrorKind;
  topic?: string;
  cause?: unknown;
  retrying: boolean;
}

export interface RealtimeTransport {
  subscribe<P = unknown>(
    topic: string,
    cb: SubscriptionCallback<P>,
  ): Unsubscribe;
  publish<P = unknown>(
    topic: string,
    event: Omit<RealtimeEvent<P>, "topic">,
  ): Promise<void>;
  connect?(): void;
  disconnect?(): void;
  onReconnect?(cb: () => void): Unsubscribe;
  onStatusChange?(cb: (status: RealtimeConnectionStatus) => void): Unsubscribe;
}

export interface LockProvider<R extends string = string> {
  lock(resource: R, params: LockParams): Promise<Lock>;
  unlock(resource: R, params: UnlockParams): Promise<Lock>;
  getLock(resource: R, params: GetLockParams): Promise<Lock | null>;
  getLocks(resource: R, params?: GetLocksParams): Promise<Lock[]>;
}

export interface RealtimeDataProvider<R extends string = string>
  extends DataProvider<R> {
  subscribe<P = unknown>(
    topic: string,
    cb: SubscriptionCallback<P>,
  ): Unsubscribe;
  publish<P = unknown>(
    topic: string,
    event: Omit<RealtimeEvent<P>, "topic">,
  ): Promise<void>;
  lock(resource: R, params: LockParams): Promise<Lock>;
  unlock(resource: R, params: UnlockParams): Promise<Lock>;
  getLock(resource: R, params: GetLockParams): Promise<Lock | null>;
  getLocks(resource: R, params?: GetLocksParams): Promise<Lock[]>;
  onReconnect?(cb: () => void): Unsubscribe;
  onStatusChange?(cb: (status: RealtimeConnectionStatus) => void): Unsubscribe;
}

export interface RealtimeDataProviderOptions<R extends string = string> {
  locks?: LockProvider<R>;
}
