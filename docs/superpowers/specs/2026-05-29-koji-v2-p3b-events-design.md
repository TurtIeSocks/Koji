# Koji V2 — Phase 3b: `koji-events` (outbox + dispatcher + webhooks) design

**Date:** 2026-05-29
**Status:** Approved-by-delegation (autonomous run). The architecture (§7, §12) deferred this deep-dive — this is it, condensed. Assumptions = async review checkpoint.
**Parent:** architecture §4 (koji-events → core, db), §7 (events + subscribers). Mirrors the koji-jobs claim/lease pattern.

## Goal
Durable domain-event delivery: producers append to an **outbox** table; a **dispatcher** worker claims due rows (`SKIP LOCKED` + lease, like koji-jobs), delivers to matching **subscribers**, retries with exponential backoff, dead-letters after `max_attempts`. Generic `WebhookSubscriber` (HTTP POST + HMAC) reads a `webhook_subscription` registry. Additive, not wired (the `DragoniteSubscriber` + event emission wire in P4/P5).

## Schema (koji-migration)

```sql
CREATE TABLE event_outbox (
  id              BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  public_id       CHAR(26)    NOT NULL UNIQUE,      -- ULID
  topic           VARCHAR(64) NOT NULL,             -- 'area.route_updated' | 'area.geofence_updated'
  payload         JSON        NOT NULL,
  status          ENUM('pending','delivering','delivered','dead') NOT NULL DEFAULT 'pending',
  attempts        INT         NOT NULL DEFAULT 0,
  max_attempts    INT         NOT NULL DEFAULT 8,
  next_attempt_at DATETIME    NOT NULL,             -- backoff schedule; due when <= NOW()
  locked_by       VARCHAR(64) NULL,
  lease_expires   DATETIME    NULL,
  last_error      TEXT        NULL,
  created_at      DATETIME    NOT NULL,
  delivered_at    DATETIME    NULL,
  INDEX idx_due (status, next_attempt_at),
  INDEX idx_topic (topic)
);

CREATE TABLE webhook_subscription (
  id          BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  url         VARCHAR(512) NOT NULL,
  secret      VARCHAR(255) NULL,                    -- HMAC-SHA256 signing key
  topics      JSON         NOT NULL,                -- ["area.route_updated", ...]; empty = all
  active      TINYINT(1)   NOT NULL DEFAULT 1,
  created_at  DATETIME     NOT NULL,
  updated_at  DATETIME     NOT NULL
);
```

## Dispatcher semantics (decided)
- **Per-event delivery** (not per-(event,subscriber)): the dispatcher claims one due event, delivers to **all** matching active subscribers (topic filter; empty `topics` = all), and the event succeeds only if **every** matching subscriber accepts (2xx). Any failure → backoff `next_attempt_at = NOW() + min(2^attempts sec, cap)`, `attempts+1`; `attempts >= max_attempts` → `dead`.
- **Idempotency on the receiver:** since a retry re-delivers to already-succeeded subscribers, each POST carries the event `public_id` (header `X-Koji-Event-Id`) + HMAC signature (`X-Koji-Signature: sha256=<hex>` over the raw body) so receivers dedup. (Per-delivery tracking is a future refinement; per-event keeps the first cut simple — matches the koji-jobs single-row claim model.)
- **Claim:** identical shape to koji-jobs §5 — `SELECT ... WHERE (status='pending' OR (status='delivering' AND lease_expires<NOW())) AND next_attempt_at<=NOW() AND attempts<max_attempts ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED`, then `UPDATE status='delivering', locked_by, lease_expires=NOW()+60s`.
- Lease 60s, heartbeat 20s (reuse the koji-jobs cadence).

## Crate surface (`koji-events`, → koji-core + sea-orm + reqwest)
```rust
pub trait Subscriber: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    async fn deliver(&self, event: &Event) -> Result<(), DeliverError>;   // async-trait
    fn interested_in(&self, topic: &str) -> bool { true }
}
pub struct Event { pub id: String /*ULID*/, pub topic: String, pub payload: serde_json::Value }

pub struct WebhookSubscriber { http: reqwest::Client, db: DatabaseConnection }   // reads webhook_subscription, HMAC-signs
pub struct EventDispatcher { db, subscribers: Vec<Arc<dyn Subscriber>>, ... }
impl EventDispatcher {
    pub async fn publish(db, topic: &str, payload: &impl Serialize) -> Result<EventId>;  // append to outbox
    pub fn spawn(self: Arc<Self>) -> DispatcherHandle;                                    // claim→deliver→backoff loop
}
```
`publish` is callable without the dispatcher running (pure outbox append) — so producers (calc-persist, `POST /:id/publish`) emit events even if delivery is async/later.

## Sub-commit plan
1. **Migrations** — `event_outbox` + `webhook_subscription` (raw MySQL DDL, like the job table). Green.
2. **koji-events crate** — entities + Event/Subscriber/DeliverError + WebhookSubscriber (HMAC) + EventDispatcher (publish + claim/deliver/backoff loop). `cargo build -p koji-events` green; DB-free unit tests (HMAC signature stability, backoff schedule). Additive, unwired. Commit.

## Assumptions (DELEGATE CHECKPOINT)
1. **Per-event (not per-subscriber) delivery + receiver-side idempotency via `X-Koji-Event-Id` + HMAC.** Per-delivery tracking deferred (simplest first cut; matches koji-jobs claim model). Revisit if a flaky subscriber starves others.
2. **`topics` as a JSON array; empty = subscribe-all.**
3. **HMAC-SHA256 over the raw JSON body**, hex, header `X-Koji-Signature: sha256=…`. (Matches common webhook conventions; confirm against any existing Koji webhook consumers.)
4. **Backoff** `2^attempts` seconds capped (e.g. 1h), `max_attempts` default 8 → ~ up to several hours before dead-letter.
5. **`DragoniteSubscriber` + actual event emission are P4/P5** — this crate ships generic + unwired; runtime (claim/deliver/backoff) is maintainer-verified against a real DB (no DB here).
6. **reqwest** for the webhook HTTP (already a workspace dep).
