# Koji Admin V2 Foundation — Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute this plan. Each task is an independently-reviewable unit; dispatch one implementer per task, TDD throughout, commit on green. Steps use checkbox (- [ ]) syntax.

**Goal:** Stand up the private, auth-gated `/internal` actix surface (forward-aliases + a bespoke geofence row-list + a `actix-ws` realtime hub) and make every resource mutation and job lifecycle write emit the contract's realtime events, so the Plan B shadmin client has a working backend to consume.

**Architecture:** A new `/internal` actix scope is mounted alongside `/api/v2` behind the *same* `public_validator` auth middleware; identical endpoints register the same handlers under both scopes (no HTTP redirect), and `GET /internal/geofences` is the one bespoke handler, reshaping the existing `geofence::Query::paginate` rows into the contract `GeofenceRow`. A `RealtimeHub` (one `tokio::broadcast` of `(topic, ServerEvent)` + per-connection topic filter) lives in app state; the `koji_resource!` macro and the geofence/route handlers publish `resource/{name}` events on every mutation, and the job worker publishes `jobs`/`jobs/{id}` events through a `JobEventSink` trait injected into `JobQueue`.

**Tech Stack:** Rust 2024 edition · actix-web 4.13 · actix-ws (WS upgrade, non-actor) · actix-session 0.11 · actix-web-httpauth 0.8 · tokio 1.52 (broadcast) · futures-util · sea-orm (tests) · awc (WS test client) · serde/serde_json · utoipa 5.5.

## Global Constraints

- **Edition / toolchain:** Rust edition `2024`; workspace builds with stable; no nightly.
- **Version floors:** `actix-web = "4.13.0"`, `actix-ws = "0.3.0"`, `tokio = "1.52.3"` (feature `sync` for `broadcast`), `futures-util = "0.3"`, `awc = "3"` (dev-dep, WS test client).
- **Dependency centralization:** every new crate goes in root `Cargo.toml` `[workspace.dependencies]` first, then referenced as `{ workspace = true }` (per `534c62d`). Reconcile `Cargo.lock` in the same commit.
- **Envelope (unchanged):** `{ "status":"ok", "data":<T>, "meta"?:<Meta> }` / `{ "status":"error", "error":{…} }`. `Meta = { total, page, per_page, total_pages, has_next, has_prev }`, 1-based page, `per_page` clamped `[1,500]`. Built via `crate::utils::api_response::{ApiResponse, Meta}`.
- **Auth gate:** the `/internal` scope is wrapped by `HttpAuthentication::with_fn(auth::public_validator)` — the *same* fn that wraps `/v2` (`crates/koji-service/src/utils/auth.rs:28`). Same-origin only; no CORS. `/internal` is NOT added to `utils::openapi::ApiDoc`.
- **GeofenceRow shape (exact):** `{ id: number, name: string, mode: string, parent: number|null, geo_type: string, projects: number[], property_count: number }`.
- **WS frames (exact):** Client `{op:"subscribe"|"unsubscribe"|"publish"|"ping", topic?, event?}`; Server `{op:"pong"}` for ping, else `{topic, type, payload?, meta?}` (topic & type REQUIRED).
- **Topic names (exact):** `resource/{name}`, `resource/{name}/{id}`, `lock/{name}`, `lock/{name}/{id}`, `jobs`, `jobs/{id}`. `{name}` ∈ `geofence | route | project | property | tileserver | plugins`.
- **Event payloads (exact, mirror shadmin `addEventsForMutations`):** created → `resource/{name}` `{type:"created", payload:{ids:[id]}}`; updated → `resource/{name}/{id}` `{type:"updated", payload:{id, data}}` **and** `resource/{name}` `{type:"updated", payload:{ids:[id]}}`; deleted → `resource/{name}/{id}` `{type:"deleted", payload:{id}}` **and** `resource/{name}` `{type:"deleted", payload:{ids:[id]}}`. Jobs: `jobs/{id}` `{type:"progress"|"status", payload:{id, status, progress, phase?}}`; `jobs` `{type:"updated", payload:{id, status}}`.
- **DB-test gating:** every test needing a live DB starts with `let Some(db) = test_db().await else { return };` reading `KOJI_DB_URL` (reconstruct `.env.test` from `.env.example`; copy into any worktree — gitignored). No-env `cargo test` passes by skipping.
- **Resource-name mapping:** the react-admin Resource name is singular (`geofence`), but the macro `module` idents are `project`/`property`/`tile_server`. The topic `{name}` for the macro resources is `project`/`property`/`tileserver` (note: `tile_server` module → `tileserver` topic). Define this mapping once in the macro invocation (a `topic:` key, Task 7).

---

## File Structure

| File | Create/Modify | Responsibility |
|---|---|---|
| `Cargo.toml` (root) | Modify | Add `actix-ws`, `futures-util`, `awc` to `[workspace.dependencies]`. |
| `crates/koji-service/Cargo.toml` | Modify | Pull `actix-ws`, `futures-util` (dep) + `awc` (dev-dep). |
| `crates/koji-service/src/internal/mod.rs` | Create | The `/internal` scope builder: forward-aliases + bespoke geofence row-list + realtime WS route. |
| `crates/koji-service/src/internal/geofences.rs` | Create | Bespoke `GET /internal/geofences` row-list handler (`GeofenceRow` + `Meta`). |
| `crates/koji-service/src/internal/realtime/mod.rs` | Create | `RealtimeHub` (broadcast + topic filter), `ClientFrame`/`ServerFrame`/`ServerEvent` types, the WS handler + session-auth gate. |
| `crates/koji-service/src/internal/realtime/topics.rs` | Create | Topic constructors (`resource_topic`, `record_topic`, `jobs_topic`, etc.) + `ServerEvent` builders for created/updated/deleted. |
| `crates/koji-service/src/lib.rs` | Modify | `mod internal;` + mount `/internal` scope under the same auth; build `RealtimeHub` into app state; wire the hub as `JobQueue`'s event sink. |
| `crates/koji-service/src/public/v2/geofences.rs` | Modify | Emit resource events in `create`/`update`/`remove` (take `web::Data<RealtimeHub>`). |
| `crates/koji-service/src/public/v2/routes.rs` | Modify | Emit resource events in `create`/`update`/`remove`. |
| `crates/macros/src/lib.rs` | Modify | Thread `sortBy/order/q`+filters into `list` (replace hardcoded `AdminReqParsed`); add `topic:` key; emit resource events in `create`/`update`/`remove`. |
| `crates/koji-service/src/public/v2/resources.rs` | Modify | Add `topic:` to each `koji_resource!` invocation. |
| `crates/koji-jobs/src/types.rs` | Modify | `JobEventSink` trait; `ProgressHandle` carries an optional sink, emits on `set`. |
| `crates/koji-jobs/src/queue.rs` | Modify | `JobQueue` carries `Option<Arc<dyn JobEventSink>>`; `with_event_sink` builder. |
| `crates/koji-jobs/src/worker.rs` | Modify | Emit status events on claim (`running`) + terminal (`succeeded`/`failed`/`canceled`). |
| `crates/koji-service/tests/internal_forwards.rs` | Create | Forward-alias auth + shape-parity integration tests. |
| `crates/koji-service/tests/internal_geofences_rows.rs` | Create | Row-list pagination/sort/filter/q integration tests. |
| `crates/koji-service/tests/internal_realtime_ws.rs` | Create | WS client: subscribe→delivery, ping→pong, mutation→frame. |
| `crates/macros/tests/koji_resource.rs` | Modify | Macro list honors sort/order/q (compile + unit-level). |

---

### Task 1: Add WS + futures deps to the workspace

**Files:**
- Modify: `Cargo.toml` (root `[workspace.dependencies]`)
- Modify: `crates/koji-service/Cargo.toml`
- Test: `crates/koji-service/tests/dep_smoke.rs` (Create, temporary — deleted in Task 6's commit)

**Interfaces:**
- Produces: `actix_ws` (crate), `futures_util` (crate), `awc` (dev) available in `koji-service`.
- Consumes: nothing.

- [ ] **Step 1: Write a failing build-smoke test.** Create `crates/koji-service/tests/dep_smoke.rs`:
  ```rust
  //! Temporary: proves the new deps link. Deleted once Task 6 wires the real hub.
  #[test]
  fn actix_ws_and_futures_link() {
      // Referencing the crate roots forces a link error if the dep is missing.
      let _ = std::any::type_name::<actix_ws::Session>();
      fn _uses_futures<S: futures_util::Stream>(_s: S) {}
  }
  ```
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p koji-service --test dep_smoke 2>&1 | tail -20` → expect `error[E0432]: unresolved import \`actix_ws\`` (and `futures_util`).
- [ ] **Step 3: Add the workspace deps.** In root `Cargo.toml` `[workspace.dependencies]` add:
  ```toml
  actix-ws    = "0.3.0"
  futures-util = "0.3.32"
  awc         = { version = "3.7.0", features = ["rustls"] }
  ```
- [ ] **Step 4: Reference them in `koji-service`.** In `crates/koji-service/Cargo.toml` `[dependencies]` add `actix-ws = { workspace = true }` and `futures-util = { workspace = true }`; in `[dev-dependencies]` add `awc = { workspace = true }`.
- [ ] **Step 5: Run it, expect PASS.** `cargo test -p koji-service --test dep_smoke 2>&1 | tail -5` → expect `test actix_ws_and_futures_link ... ok`. Confirm `Cargo.lock` updated: `git status --porcelain Cargo.lock` shows ` M Cargo.lock`.
- [ ] **Step 6: Commit.** `git add Cargo.toml Cargo.lock crates/koji-service/Cargo.toml crates/koji-service/tests/dep_smoke.rs && git commit -m "build(koji-service): add actix-ws + futures-util deps (awc dev) for /internal realtime

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 2: RealtimeHub + WS frame types + topic helpers

**Files:**
- Create: `crates/koji-service/src/internal/realtime/topics.rs`
- Create: `crates/koji-service/src/internal/realtime/mod.rs`
- Create: `crates/koji-service/src/internal/mod.rs` (stub `pub(crate) mod realtime;` only this task)
- Modify: `crates/koji-service/src/lib.rs` (add `mod internal;`)
- Test: inline `#[cfg(test)] mod tests` in `realtime/mod.rs` + `realtime/topics.rs`

**Interfaces:**
- Produces:
  - `pub struct RealtimeHub` with `pub fn new() -> Self`, `pub fn publish(&self, topic: &str, event: ServerEvent)`, `pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<(String, ServerEvent)>`.
  - `pub struct ServerEvent { pub r#type: String, pub payload: Option<serde_json::Value>, pub meta: Option<serde_json::Value> }` (Clone, Serialize).
  - `enum ClientFrame { Subscribe{topic}, Unsubscribe{topic}, Publish{topic, event}, Ping }` (Deserialize, `#[serde(tag="op", rename_all="lowercase")]`).
  - `topics::{resource_topic(name)->String, record_topic(name,id)->String, lock_topic(name)->String, lock_record_topic(name,id)->String, jobs_topic()->&'static str, job_topic(id)->String}`.
  - `topics::{created(name,id), updated(name,id,data), deleted(name,id)}` → returns the `Vec<(topic, ServerEvent)>` pairs per contract §4.
- Consumes: `serde_json::Value`, `tokio::sync::broadcast`.

- [ ] **Step 1: Failing test for topic constructors.** Create `crates/koji-service/src/internal/realtime/topics.rs` with only a test module:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      #[test]
      fn topic_strings_match_contract() {
          assert_eq!(resource_topic("geofence"), "resource/geofence");
          assert_eq!(record_topic("geofence", 7), "resource/geofence/7");
          assert_eq!(lock_topic("route"), "lock/route");
          assert_eq!(lock_record_topic("route", 3), "lock/route/3");
          assert_eq!(jobs_topic(), "jobs");
          assert_eq!(job_topic("01J..."), "jobs/01J...");
      }
      #[test]
      fn created_emits_collection_only() {
          let pairs = created("geofence", 5);
          assert_eq!(pairs.len(), 1);
          assert_eq!(pairs[0].0, "resource/geofence");
          assert_eq!(pairs[0].1.r#type, "created");
          assert_eq!(pairs[0].1.payload.as_ref().unwrap()["ids"], serde_json::json!([5]));
      }
      #[test]
      fn updated_emits_record_then_collection() {
          let pairs = updated("route", 9, serde_json::json!({"id":9,"name":"x"}));
          assert_eq!(pairs[0].0, "resource/route/9");
          assert_eq!(pairs[0].1.r#type, "updated");
          assert_eq!(pairs[0].1.payload.as_ref().unwrap()["id"], 9);
          assert_eq!(pairs[0].1.payload.as_ref().unwrap()["data"]["name"], "x");
          assert_eq!(pairs[1].0, "resource/route");
          assert_eq!(pairs[1].1.payload.as_ref().unwrap()["ids"], serde_json::json!([9]));
      }
      #[test]
      fn deleted_emits_record_then_collection() {
          let pairs = deleted("project", 2);
          assert_eq!(pairs[0].0, "resource/project/2");
          assert_eq!(pairs[0].1.r#type, "deleted");
          assert_eq!(pairs[0].1.payload.as_ref().unwrap()["id"], 2);
          assert_eq!(pairs[1].0, "resource/project");
          assert_eq!(pairs[1].1.payload.as_ref().unwrap()["ids"], serde_json::json!([2]));
      }
  }
  ```
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p koji-service realtime::topics 2>&1 | tail -20` → expect `cannot find function \`resource_topic\`` (the module isn't compiled yet — also `mod` not declared). It must not yet pass.
- [ ] **Step 3: Implement the topic helpers + `ServerEvent`.** Prepend to `topics.rs`:
  ```rust
  //! Topic constructors + server-event builders. Mirrors shadmin `topics.ts` +
  //! `addEventsForMutations` payloads exactly (contract §4).
  use serde::Serialize;

  #[derive(Debug, Clone, Serialize)]
  pub struct ServerEvent {
      pub r#type: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub payload: Option<serde_json::Value>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub meta: Option<serde_json::Value>,
  }
  impl ServerEvent {
      pub fn new(r#type: impl Into<String>, payload: serde_json::Value) -> Self {
          ServerEvent { r#type: r#type.into(), payload: Some(payload), meta: None }
      }
  }
  pub fn resource_topic(name: &str) -> String { format!("resource/{name}") }
  pub fn record_topic(name: &str, id: impl std::fmt::Display) -> String { format!("resource/{name}/{id}") }
  pub fn lock_topic(name: &str) -> String { format!("lock/{name}") }
  pub fn lock_record_topic(name: &str, id: impl std::fmt::Display) -> String { format!("lock/{name}/{id}") }
  pub fn jobs_topic() -> &'static str { "jobs" }
  pub fn job_topic(id: impl std::fmt::Display) -> String { format!("jobs/{id}") }

  /// created → collection event only (`{type:"created", payload:{ids:[id]}}`).
  pub fn created(name: &str, id: impl std::fmt::Display + Clone) -> Vec<(String, ServerEvent)> {
      vec![(resource_topic(name), ServerEvent::new("created", serde_json::json!({"ids":[id.to_string().parse::<i64>().unwrap_or(0)]})))]
  }
  /// updated → record event then collection event.
  pub fn updated(name: &str, id: i64, data: serde_json::Value) -> Vec<(String, ServerEvent)> {
      vec![
          (record_topic(name, id), ServerEvent::new("updated", serde_json::json!({"id":id, "data":data}))),
          (resource_topic(name), ServerEvent::new("updated", serde_json::json!({"ids":[id]}))),
      ]
  }
  /// deleted → record event then collection event.
  pub fn deleted(name: &str, id: i64) -> Vec<(String, ServerEvent)> {
      vec![
          (record_topic(name, id), ServerEvent::new("deleted", serde_json::json!({"id":id}))),
          (resource_topic(name), ServerEvent::new("deleted", serde_json::json!({"ids":[id]}))),
      ]
  }
  ```
  Note: `created` takes the id as `Display`; the test passes an `i64` literal so `to_string().parse` round-trips. Change the test's `created("geofence", 5)` literal to `5i64`.
- [ ] **Step 4: Wire the module tree.** Create `crates/koji-service/src/internal/realtime/mod.rs` with `pub mod topics;` (rest added next step). Create `crates/koji-service/src/internal/mod.rs` with `pub(crate) mod realtime;`. Add `mod internal;` to `crates/koji-service/src/lib.rs` (alongside `mod public;`).
- [ ] **Step 5: Run it, expect PASS.** `cargo test -p koji-service realtime::topics 2>&1 | tail -8` → 4 tests `ok`.
- [ ] **Step 6: Failing test for the hub.** Append to `realtime/mod.rs` a test module:
  ```rust
  #[cfg(test)]
  mod hub_tests {
      use super::*;
      #[tokio::test]
      async fn publish_reaches_a_live_subscriber() {
          let hub = RealtimeHub::new();
          let mut rx = hub.subscribe();
          hub.publish("resource/geofence", topics::ServerEvent::new("created", serde_json::json!({"ids":[1]})));
          let (topic, ev) = rx.recv().await.unwrap();
          assert_eq!(topic, "resource/geofence");
          assert_eq!(ev.r#type, "created");
      }
      #[tokio::test]
      async fn publish_with_no_subscribers_does_not_panic() {
          let hub = RealtimeHub::new();
          hub.publish("jobs", topics::ServerEvent::new("updated", serde_json::json!({"id":"x","status":"queued"})));
      }
  }
  ```
- [ ] **Step 7: Run it, expect FAIL.** `cargo test -p koji-service realtime::mod 2>&1 | tail -20` → expect `cannot find type \`RealtimeHub\``.
- [ ] **Step 8: Implement `RealtimeHub` + `ClientFrame`.** Prepend to `realtime/mod.rs`:
  ```rust
  //! In-process realtime pub/sub hub + the `GET /internal/realtime` WS handler.
  //! ponytail: ONE `tokio::broadcast` of `(topic, ServerEvent)` + a per-connection
  //! subscribed-topic filter. Shard to a `DashMap<topic, …>` only if connection
  //! count ever justifies it.
  pub mod topics;
  pub use topics::ServerEvent;

  use serde::Deserialize;
  use tokio::sync::broadcast;

  const CHANNEL_CAP: usize = 1024;

  #[derive(Clone)]
  pub struct RealtimeHub {
      tx: broadcast::Sender<(String, ServerEvent)>,
  }
  impl RealtimeHub {
      pub fn new() -> Self {
          let (tx, _rx) = broadcast::channel(CHANNEL_CAP);
          RealtimeHub { tx }
      }
      /// Broadcast an event on a topic. Lossy iff a slow subscriber lags > CHANNEL_CAP
      /// (broadcast drops oldest for that receiver — acceptable: client refetches on
      /// reconnect). A send with zero receivers is a no-op (returns Err, ignored).
      pub fn publish(&self, topic: &str, event: ServerEvent) {
          let _ = self.tx.send((topic.to_string(), event));
      }
      pub fn subscribe(&self) -> broadcast::Receiver<(String, ServerEvent)> {
          self.tx.subscribe()
      }
  }
  impl Default for RealtimeHub { fn default() -> Self { Self::new() } }

  /// Client → server frames (contract §3). `#[serde(tag="op")]` internally-tagged.
  #[derive(Debug, Deserialize)]
  #[serde(tag = "op", rename_all = "lowercase")]
  pub enum ClientFrame {
      Subscribe { topic: String },
      Unsubscribe { topic: String },
      Publish { topic: String, event: ServerEvent },
      Ping,
  }
  // `ServerEvent` needs Deserialize for the `publish` frame; add `#[derive(Deserialize)]`
  // to it in topics.rs alongside Serialize (and `#[serde(default)]` on payload/meta).
  ```
  Then in `topics.rs` change the `ServerEvent` derive to `#[derive(Debug, Clone, Serialize, serde::Deserialize)]` and add `#[serde(default)]` to `payload` and `meta`.
- [ ] **Step 9: Run it, expect PASS.** `cargo test -p koji-service realtime 2>&1 | tail -10` → all topic + hub tests `ok`.
- [ ] **Step 10: Commit.** `git add crates/koji-service/src/internal crates/koji-service/src/lib.rs && git rm crates/koji-service/tests/dep_smoke.rs && git commit -m "feat(internal): RealtimeHub broadcast + WS frame types + contract topic helpers

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 3: WS handler — handshake auth, frame loop, ping→pong, event forwarding

**Files:**
- Modify: `crates/koji-service/src/internal/realtime/mod.rs`
- Test: `crates/koji-service/tests/internal_realtime_ws.rs` (Create)

**Interfaces:**
- Produces: `pub(crate) async fn realtime_ws(req: HttpRequest, body: web::Payload, hub: web::Data<RealtimeHub>, session: Session) -> Result<HttpResponse, actix_web::Error>` — the WS upgrade handler. Rejects (`401`) unauthenticated handshakes (no session `logged_in` AND `KOJI_SECRET` non-empty AND no valid bearer query/subprotocol). Spawns a task: reads `ClientFrame`s, maintains a `HashSet<String>` of subscribed topics, forwards matching hub `(topic, ServerEvent)` as `{topic,type,payload,meta}`, answers `Ping` with `{op:"pong"}`.
- Consumes: `RealtimeHub` (app state), `actix_session::Session`, `actix_ws::handle`.

- [ ] **Step 1: Failing WS integration test (subscribe→delivery + ping/pong).** Create `crates/koji-service/tests/internal_realtime_ws.rs`. (This test boots a real `actix_web::test::start` server because `actix-ws` needs a live TCP listener; `awc` is the WS client. Auth is satisfied with `KOJI_SECRET` unset → open mode.)
  ```rust
  //! WS hub integration: subscribe→delivery, ping→pong, mutation→frame.
  //! No DB required for ping/pong + manual publish (uses the hub directly via a
  //! test-only publish route). The mutation-publish path is covered in
  //! `internal_geofences_rows.rs` (DB-gated). Auth: `KOJI_SECRET` unset = open.
  use actix_web::{web, App, HttpServer};
  use futures_util::{SinkExt, StreamExt};

  #[actix_web::test]
  async fn ping_gets_pong_and_subscribed_event_is_delivered() {
      // SAFETY: test-only env mutation; serialized by being the sole test touching it.
      unsafe { std::env::set_var("KOJI_SECRET", ""); }
      let hub = koji_service::test_realtime_hub();
      let hub_data = web::Data::new(hub.clone());
      let srv = actix_web::test::start(move || {
          App::new()
              .app_data(hub_data.clone())
              .wrap(actix_session::SessionMiddleware::builder(
                  actix_session::storage::CookieSessionStore::default(),
                  actix_web::cookie::Key::from(&[0u8;64]),
              ).cookie_secure(false).build())
              .route("/internal/realtime", web::get().to(koji_service::test_realtime_ws()))
      });
      let url = srv.url("/internal/realtime").replace("http://", "ws://");
      let (_resp, mut conn) = awc::Client::new().ws(url).connect().await.unwrap();

      // ping → pong
      conn.send(awc::ws::Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap();
      let msg = conn.next().await.unwrap().unwrap();
      assert!(matches!(msg, awc::ws::Frame::Text(ref b) if b.starts_with(b"{\"op\":\"pong\"")));

      // subscribe then publish on the hub → event frame delivered
      conn.send(awc::ws::Message::Text(r#"{"op":"subscribe","topic":"resource/geofence"}"#.into())).await.unwrap();
      // give the reader task a tick to register the subscription
      tokio::time::sleep(std::time::Duration::from_millis(50)).await;
      hub.publish("resource/geofence", koji_service::test_server_event("created", serde_json::json!({"ids":[1]})));
      let msg = conn.next().await.unwrap().unwrap();
      let awc::ws::Frame::Text(bytes) = msg else { panic!("expected text frame") };
      let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
      assert_eq!(v["topic"], "resource/geofence");
      assert_eq!(v["type"], "created");
      assert_eq!(v["payload"]["ids"], serde_json::json!([1]));
  }

  #[actix_web::test]
  async fn unsubscribed_topic_is_not_delivered() {
      unsafe { std::env::set_var("KOJI_SECRET", ""); }
      let hub = koji_service::test_realtime_hub();
      let hub_data = web::Data::new(hub.clone());
      let srv = actix_web::test::start(move || {
          App::new().app_data(hub_data.clone())
              .wrap(actix_session::SessionMiddleware::builder(
                  actix_session::storage::CookieSessionStore::default(),
                  actix_web::cookie::Key::from(&[0u8;64]),
              ).cookie_secure(false).build())
              .route("/internal/realtime", web::get().to(koji_service::test_realtime_ws()))
      });
      let url = srv.url("/internal/realtime").replace("http://","ws://");
      let (_r, mut conn) = awc::Client::new().ws(url).connect().await.unwrap();
      conn.send(awc::ws::Message::Text(r#"{"op":"subscribe","topic":"jobs"}"#.into())).await.unwrap();
      tokio::time::sleep(std::time::Duration::from_millis(50)).await;
      hub.publish("resource/geofence", koji_service::test_server_event("created", serde_json::json!({"ids":[1]})));
      // round-trip a ping; the only frame we should see back is the pong, not the event.
      conn.send(awc::ws::Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap();
      let msg = conn.next().await.unwrap().unwrap();
      let awc::ws::Frame::Text(bytes) = msg else { panic!("text") };
      assert!(bytes.starts_with(b"{\"op\":\"pong\""), "got {:?}", bytes);
  }
  ```
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p koji-service --test internal_realtime_ws 2>&1 | tail -20` → expect `cannot find function \`test_realtime_ws\` in crate \`koji_service\``.
- [ ] **Step 3: Implement the WS handler.** Append to `realtime/mod.rs`:
  ```rust
  use actix_session::SessionExt;
  use actix_web::{web, HttpRequest, HttpResponse};
  use std::collections::HashSet;
  use futures_util::StreamExt;

  /// Auth gate for the WS upgrade: mirror `public_validator` (session `logged_in`
  /// OR empty `KOJI_SECRET` OR `Bearer`/`?token=` == secret). Same-origin only.
  fn ws_authorized(req: &HttpRequest) -> bool {
      let session = req.get_session();
      if session.get::<bool>("logged_in").ok().flatten().unwrap_or(false) { return true; }
      let secret = std::env::var("KOJI_SECRET").unwrap_or_default();
      if secret.is_empty() { return true; }
      // bearer via query (?token=) — browsers can't set WS headers; subprotocol is
      // the shadmin transport's path, query is the simpler fallback.
      if let Some(tok) = req.query_string().split('&').find_map(|kv| kv.strip_prefix("token=")) {
          if crate::utils::auth::ct_eq(tok, &secret) { return true; }
      }
      false
  }

  pub(crate) async fn realtime_ws(
      req: HttpRequest,
      body: web::Payload,
      hub: web::Data<RealtimeHub>,
  ) -> Result<HttpResponse, actix_web::Error> {
      if !ws_authorized(&req) {
          return Ok(HttpResponse::Unauthorized().finish());
      }
      let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
      let mut rx = hub.subscribe();
      actix_web::rt::spawn(async move {
          let mut topics: HashSet<String> = HashSet::new();
          loop {
              tokio::select! {
                  // inbound client frames
                  Some(Ok(msg)) = msg_stream.next() => {
                      match msg {
                          actix_ws::Message::Text(txt) => {
                              if let Ok(frame) = serde_json::from_str::<ClientFrame>(&txt) {
                                  match frame {
                                      ClientFrame::Subscribe { topic } => { topics.insert(topic); }
                                      ClientFrame::Unsubscribe { topic } => { topics.remove(&topic); }
                                      ClientFrame::Ping => {
                                          if session.text(r#"{"op":"pong"}"#).await.is_err() { break; }
                                      }
                                      ClientFrame::Publish { topic, event } => {
                                          hub.publish(&topic, event); // re-broadcast (protocol completeness)
                                      }
                                  }
                              }
                          }
                          actix_ws::Message::Ping(bytes) => { let _ = session.pong(&bytes).await; }
                          actix_ws::Message::Close(_) => break,
                          _ => {}
                      }
                  }
                  // outbound hub events
                  ev = rx.recv() => {
                      match ev {
                          Ok((topic, event)) => {
                              if topics.contains(&topic) {
                                  let frame = serde_json::json!({
                                      "topic": topic, "type": event.r#type,
                                      "payload": event.payload, "meta": event.meta,
                                  });
                                  if session.text(frame.to_string()).await.is_err() { break; }
                              }
                          }
                          Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                          Err(_) => break,
                      }
                  }
                  else => break,
              }
          }
          let _ = session.close(None).await;
      });
      Ok(response)
  }
  ```
  Make `ct_eq` reachable: in `crates/koji-service/src/utils/auth.rs` change `pub(crate) fn ct_eq` (already `pub(crate)` — confirm; no edit needed).
- [ ] **Step 4: Add the test-only re-exports.** In `crates/koji-service/src/lib.rs` (alongside the other `#[doc(hidden)] pub fn` test helpers ~line 30-113) add:
  ```rust
  /// Test surface: a fresh `RealtimeHub`.
  #[doc(hidden)]
  pub fn test_realtime_hub() -> internal::realtime::RealtimeHub { internal::realtime::RealtimeHub::new() }
  /// Test surface: the WS handler fn, for mounting on a `test::start` server.
  #[doc(hidden)]
  pub fn test_realtime_ws() -> fn(actix_web::HttpRequest, actix_web::web::Payload, actix_web::web::Data<internal::realtime::RealtimeHub>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<actix_web::HttpResponse, actix_web::Error>>>> {
      |req, body, hub| Box::pin(internal::realtime::realtime_ws(req, body, hub))
  }
  /// Test surface: build a `ServerEvent`.
  #[doc(hidden)]
  pub fn test_server_event(t: &str, payload: serde_json::Value) -> internal::realtime::ServerEvent {
      internal::realtime::ServerEvent::new(t, payload)
  }
  ```
  Make the module path reachable: `internal` is `mod internal;` (private) — these `pub fn`s expose only the needed items, so keep `internal` private. Ensure `internal::realtime` items used here (`RealtimeHub`, `realtime_ws`, `ServerEvent`) are `pub`/`pub(crate)` as written.
- [ ] **Step 5: Run it, expect PASS.** `cargo test -p koji-service --test internal_realtime_ws 2>&1 | tail -15` → both tests `ok`. (Run with `run_in_background: true` — boots a TCP server, may take >5s cold.)
- [ ] **Step 6: Commit.** `git add crates/koji-service/src/internal/realtime/mod.rs crates/koji-service/src/lib.rs crates/koji-service/tests/internal_realtime_ws.rs && git commit -m "feat(internal): actix-ws realtime handler — handshake auth, subscribe filter, ping/pong, event forwarding

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 4: Bespoke `GET /internal/geofences` row-list

**Files:**
- Create: `crates/koji-service/src/internal/geofences.rs`
- Modify: `crates/koji-service/src/internal/mod.rs` (add `pub(crate) mod geofences;`)
- Test: `crates/koji-service/tests/internal_geofences_rows.rs` (Create)

**Interfaces:**
- Produces: `pub(crate) async fn list_rows(conn: web::Data<KojiDb>, query: web::Query<RowQuery>) -> Result<HttpResponse, ServiceError>` returning `{status:"ok", data: GeofenceRow[], meta: Meta}`.
  - `RowQuery { page, per_page, sort_by (alias "sortBy"), order, q, project, parent, geotype, mode }` — wire accepts both `sortBy` and `sort_by`.
  - `GeofenceRow { id, name, mode, parent: Option<i64>, geo_type, projects: Vec<i64>, property_count: usize }`.
- Consumes: `koji_db::db::geofence::Query::paginate(&conn.koji, AdminReqParsed)` (existing — `crates/koji-db/src/db/geofence/list.rs:19`), `crate::utils::api_response::{ApiResponse, Meta}`.

- [ ] **Step 1: Failing DB integration test for the row shape + pagination.** Create `crates/koji-service/tests/internal_geofences_rows.rs` (mirror `v2_db.rs`'s `test_db`/`serial_guard`/`unique_name`/`body_json`/`cleanup_geofence` helpers — copy them in). Use `koji_service::test_internal_geofences_app(db)`:
  ```rust
  #![allow(clippy::await_holding_lock)]
  use actix_web::test;
  use koji_db::KojiDb;
  use koji_jobs::JobId;
  use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
  use std::sync::{Mutex, MutexGuard};

  static SERIAL: Mutex<()> = Mutex::new(());
  fn serial_guard() -> MutexGuard<'static, ()> { SERIAL.lock().unwrap_or_else(|p| p.into_inner()) }
  async fn test_db() -> Option<DatabaseConnection> {
      let Ok(url) = std::env::var("KOJI_DB_URL") else { eprintln!("skip: KOJI_DB_URL unset"); return None; };
      Database::connect(&url).await.ok().or_else(|| { eprintln!("skip: connect failed"); None })
  }
  fn unique_name(tag: &str) -> String { format!("test-{tag}-{}", JobId::new().as_string()) }
  async fn body_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
      serde_json::from_slice(&test::read_body(resp).await).unwrap()
  }
  async fn insert_geofence(db: &DatabaseConnection, name: &str, mode: &str) -> u64 {
      db.execute(Statement::from_sql_and_values(DbBackend::MySql,
          "INSERT INTO geofence (name, mode, geo_type, geometry, created_at, updated_at) \
           VALUES (?, ?, 'Polygon', '{\"type\":\"Polygon\",\"coordinates\":[[[0,0],[1,0],[1,1],[0,0]]]}', NOW(), NOW())",
          [Value::from(name.to_owned()), Value::from(mode.to_owned())])).await.unwrap();
      db.query_one(Statement::from_string(DbBackend::MySql, "SELECT LAST_INSERT_ID() AS id"))
          .await.unwrap().unwrap().try_get::<u64>("", "id").unwrap()
  }
  async fn cleanup(db: &DatabaseConnection, id: u64) {
      let _ = db.execute(Statement::from_sql_and_values(DbBackend::MySql,
          "DELETE FROM geofence WHERE id = ?", [Value::from(id)])).await;
  }

  #[actix_web::test]
  async fn row_list_returns_geofence_row_shape_and_meta() {
      let _g = serial_guard();
      let Some(conn) = test_db().await else { return; };
      let db = build_test_koji_db(conn.clone()).await;
      let name = unique_name("rows");
      let id = insert_geofence(&conn, &name, "pokemon").await;

      let app = test::init_service(koji_service::test_internal_geofences_app(db)).await;
      let req = test::TestRequest::get().uri(&format!("/internal/geofences?per_page=500&q={name}")).to_request();
      let resp = test::call_service(&app, req).await;
      assert!(resp.status().is_success());
      let v = body_json(resp).await;
      assert_eq!(v["status"], "ok");
      let row = v["data"].as_array().unwrap().iter().find(|r| r["name"] == name).unwrap();
      assert_eq!(row["id"], id);
      assert_eq!(row["mode"], "pokemon");
      assert_eq!(row["geo_type"], "Polygon");
      assert!(row["parent"].is_null());
      assert!(row["projects"].is_array());
      assert!(row["property_count"].is_u64());
      // meta block present + 1-based page
      assert_eq!(v["meta"]["page"], 1);
      assert!(v["meta"]["per_page"].as_i64().unwrap() <= 500);
      assert!(v["meta"]["total"].as_i64().unwrap() >= 1);
      cleanup(&conn, id).await;
  }

  #[actix_web::test]
  async fn row_list_honors_mode_filter_and_sort() {
      let _g = serial_guard();
      let Some(conn) = test_db().await else { return; };
      let db = build_test_koji_db(conn.clone()).await;
      let a = unique_name("aaa"); let z = unique_name("zzz");
      let ida = insert_geofence(&conn, &a, "raid").await;
      let idz = insert_geofence(&conn, &z, "quest").await;

      let app = test::init_service(koji_service::test_internal_geofences_app(db)).await;
      // mode filter: only the raid one
      let req = test::TestRequest::get().uri("/internal/geofences?per_page=500&mode=raid&sortBy=name&order=ASC").to_request();
      let v = body_json(test::call_service(&app, req).await).await;
      assert!(v["data"].as_array().unwrap().iter().all(|r| r["mode"] == "raid"));
      assert!(v["data"].as_array().unwrap().iter().any(|r| r["id"] == ida));
      assert!(v["data"].as_array().unwrap().iter().all(|r| r["id"] != idz));
      cleanup(&conn, ida).await; cleanup(&conn, idz).await;
  }
  ```
  **Note on `KojiDb` construction:** `KojiDb { koji, golbat }` (`crates/koji-db/src/lib.rs:43`) has public fields. `v2_db.rs` already has `build_test_koji_db(conn)` (`tests/v2_db.rs:93`) that points both `koji`/`golbat` at the same connection — **copy that helper verbatim into each `/internal` test file** and call `build_test_koji_db(conn.clone()).await` (as every test in this plan does). Do NOT invent a `from_existing_for_test` constructor; the struct literal `KojiDb { koji: conn.clone(), golbat: conn.clone() }` inside the copied helper is the canonical form.
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p koji-service --test internal_geofences_rows 2>&1 | tail -20` → expect `cannot find function \`test_internal_geofences_app\``.
- [ ] **Step 3: Implement the row handler.** Create `crates/koji-service/src/internal/geofences.rs`:
  ```rust
  //! Bespoke `GET /internal/geofences` — paginated flat rows for the shadmin
  //! DataTable. Reuses the public geofence list DB path
  //! (`geofence::Query::paginate` / `AdminReqParsed`) — only the serialization
  //! differs (rows, not a GeoJSON FeatureCollection). Contract §2 `GeofenceRow`.
  use actix_web::{web, HttpResponse};
  use koji_db::{KojiDb, query_args::AdminReqParsed};
  use serde::{Deserialize, Serialize};

  use crate::utils::api_response::{ApiResponse, Meta};
  use crate::utils::error::ServiceError;

  /// `?page&per_page&sortBy|sort_by&order&q` + filters `project/parent/geotype/mode`.
  #[derive(Debug, Deserialize)]
  pub(crate) struct RowQuery {
      page: Option<i64>,
      per_page: Option<i64>,
      #[serde(alias = "sortBy")]
      sort_by: Option<String>,
      order: Option<String>,
      q: Option<String>,
      project: Option<u32>,
      parent: Option<u32>,
      geotype: Option<String>,
      mode: Option<String>,
  }

  #[derive(Debug, Serialize)]
  pub(crate) struct GeofenceRow {
      id: i64,
      name: String,
      mode: String,
      parent: Option<i64>,
      geo_type: String,
      projects: Vec<i64>,
      property_count: usize,
  }

  pub(crate) async fn list_rows(
      conn: web::Data<KojiDb>,
      query: web::Query<RowQuery>,
  ) -> Result<HttpResponse, ServiceError> {
      let page = query.page.unwrap_or(1).max(1);
      let per_page = query.per_page.unwrap_or(50).clamp(1, 500);
      let args = AdminReqParsed {
          page: (page - 1) as u64,          // koji-db paginate is 0-based
          per_page: per_page as u64,
          sort_by: query.sort_by.clone().unwrap_or_else(|| "id".to_string()),
          order: query.order.clone().unwrap_or_else(|| "ASC".to_string()),
          q: query.q.clone().unwrap_or_default(),
          geotype: query.geotype.clone(),
          project: query.project,
          mode: query.mode.clone(),
          parent: query.parent,
          geofenceid: None,
          pointsmin: None,
          pointsmax: None,
      };
      let (rows, total, _hn, _hp) =
          koji_db::db::geofence::Query::paginate(&conn.koji, args).await?.into_parts();

      // `paginate` emits {id,name,mode,geo_type,parent,projects:[…],properties:[…],routes:[…]}.
      // Reshape to the contract GeofenceRow: projects → id list; properties → count.
      let data: Vec<GeofenceRow> = rows.into_iter().map(|r| {
          let projects = r.get("projects").and_then(|p| p.as_array()).map(|a| {
              a.iter().filter_map(|p| p.get("id").and_then(serde_json::Value::as_i64)).collect()
          }).unwrap_or_default();
          let property_count = r.get("properties").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
          GeofenceRow {
              id: r.get("id").and_then(serde_json::Value::as_i64).unwrap_or(0),
              name: r.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
              mode: r.get("mode").and_then(|v| v.as_str()).unwrap_or("").to_string(),
              parent: r.get("parent").and_then(serde_json::Value::as_i64),
              geo_type: r.get("geo_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
              projects, property_count,
          }
      }).collect();

      Ok(ApiResponse::success_paginated(data, Meta::build(total as i64, page, per_page)))
  }
  ```
  Add `pub(crate) mod geofences;` to `crates/koji-service/src/internal/mod.rs`. Confirm `ApiResponse::success_paginated` exists (used by the macro at `macros/src/lib.rs:841`) — yes.
- [ ] **Step 4: Add the test-only app builder.** In `lib.rs` next to `test_db_app`:
  ```rust
  /// Test surface: a minimal App mounting `GET /internal/geofences` (row list).
  #[doc(hidden)]
  pub fn test_internal_geofences_app(db: koji_db::KojiDb) -> actix_web::App<
      impl actix_web::dev::ServiceFactory<actix_web::dev::ServiceRequest, Config=(),
          Response = actix_web::dev::ServiceResponse, Error = actix_web::Error, InitError=()>> {
      App::new()
          .app_data(web::Data::new(db))
          .service(web::scope("/internal").service(
              web::resource("/geofences").route(web::get().to(internal::geofences::list_rows))))
  }
  ```
- [ ] **Step 5: Run it, expect PASS (with DB).** `set -a; source ./.env.test; set +a; cargo test -p koji-service --test internal_geofences_rows -- --nocapture 2>&1 | tail -15` → 2 tests `ok` (run in background; cold DB connect + serial). Without DB: both skip and the binary passes.
- [ ] **Step 6: Commit.** `git add crates/koji-service/src/internal crates/koji-service/src/lib.rs crates/koji-service/tests/internal_geofences_rows.rs && git commit -m "feat(internal): bespoke GET /internal/geofences row-list reusing geofence paginate path

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 5: Macro list honors sortBy/order/q + filters (public correctness fix)

**Files:**
- Modify: `crates/macros/src/lib.rs` (the `list` handler in the `koji_resource!` expansion)
- Test: `crates/macros/tests/koji_resource.rs` (Modify — add list-args test) + `crates/koji-service/tests/internal_geofences_rows.rs` (extend: macro CRUD sort/filter via `/internal/projects`)

**Interfaces:**
- Produces: the generated `list` handler now extracts a richer query (`page, per_page, sortBy, order, q` + `project/parent/geotype/mode`) and threads it into `AdminReqParsed` instead of the hardcoded `sort_by:"id"/order:"ASC"/q:""`.
- Consumes: existing `koji_db::db::#module::Query::paginate(&db.koji, AdminReqParsed)` (already supports these args — `crates/koji-db/src/db/*` `paginate`).

- [ ] **Step 1: Failing macro unit test.** In `crates/macros/tests/koji_resource.rs` add a test that the generated `list` deserializes a `?sortBy=name&order=DESC&q=foo` query into the right `AdminReqParsed` shape. Since the handler is async + DB-backed, assert at the query-extraction layer: add a `pub(crate)` `ListQuery` struct to the macro expansion and unit-test *its* deserialization here. Test:
  ```rust
  // Assumes the macro stamps a `pub(crate) struct ListQuery` per module with
  // serde aliases. We re-declare an identical struct here and assert the wire
  // contract (the macro test crate cannot reach pub(crate) items, so this guards
  // the *shape* the macro must produce).
  #[test]
  fn list_query_accepts_camel_sort_and_filters() {
      #[derive(serde::Deserialize, Default, Debug)]
      struct ListQuery {
          page: Option<i64>, per_page: Option<i64>,
          #[serde(alias = "sortBy")] sort_by: Option<String>,
          order: Option<String>, q: Option<String>,
          project: Option<u32>, parent: Option<u32>,
          geotype: Option<String>, mode: Option<String>,
      }
      let q: ListQuery = serde_urlencoded::from_str("sortBy=name&order=DESC&q=foo&parent=3").unwrap();
      assert_eq!(q.sort_by.as_deref(), Some("name"));
      assert_eq!(q.order.as_deref(), Some("DESC"));
      assert_eq!(q.q.as_deref(), Some("foo"));
      assert_eq!(q.parent, Some(3));
  }
  ```
  (Add `serde_urlencoded` as a dev-dep to `crates/macros/Cargo.toml` if absent.)
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p macros --test koji_resource list_query 2>&1 | tail -10` → expect compile error (`serde_urlencoded` missing) or, once added, this is a guard test that should already pass — instead make the *real* failing signal the integration test in Step 3. (If the guard passes immediately, that's fine — its job is to pin the wire shape; the behavioral FAIL is Step 3.)
- [ ] **Step 3: Failing integration assertion (macro CRUD server-side sort).** In `internal_geofences_rows.rs` add (DB-gated) a test using a forwarded `/internal/projects` (Task 6 mounts it; if Task 6 not yet merged, point at `/api/v2/projects` via `test_db_app` extended — but prefer ordering Task 6 before this). Test that two projects inserted `zzz`,`aaa` come back `aaa` first with `?sortBy=name&order=ASC`:
  ```rust
  #[actix_web::test]
  async fn macro_list_sorts_by_name_server_side() {
      let _g = serial_guard();
      let Some(conn) = test_db().await else { return; };
      let db = build_test_koji_db(conn.clone()).await;
      let a = unique_name("aaa"); let z = unique_name("zzz");
      conn.execute(Statement::from_sql_and_values(DbBackend::MySql,
          "INSERT INTO project (name, golbat) VALUES (?, 0), (?, 0)",
          [Value::from(z.clone()), Value::from(a.clone())])).await.unwrap();
      let app = test::init_service(koji_service::test_internal_geofences_app(db)).await; // extend builder to also mount projects::scope under /internal
      let req = test::TestRequest::get().uri(&format!("/internal/projects?per_page=500&sortBy=name&order=ASC")).to_request();
      let v = body_json(test::call_service(&app, req).await).await;
      let names: Vec<&str> = v["data"].as_array().unwrap().iter().filter_map(|r| r["name"].as_str())
          .filter(|n| *n == a || *n == z).collect();
      assert_eq!(names, vec![a.as_str(), z.as_str()], "aaa must sort before zzz");
      conn.execute(Statement::from_sql_and_values(DbBackend::MySql, "DELETE FROM project WHERE name IN (?,?)",
          [Value::from(a), Value::from(z)])).await.unwrap();
  }
  ```
  (Column is `golbat` (boolean) post scanner→golbat rename — confirm the NOT-NULL columns of `project` against `crates/koji-db/src/entity/project.rs` before writing the INSERT; add any other required columns.)
- [ ] **Step 4: Run it, expect FAIL.** `set -a; source ./.env.test; set +a; cargo test -p koji-service --test internal_geofences_rows macro_list_sorts 2>&1 | tail -15` → expect the assertion fails (rows come back id-ASC, so `zzz` (lower id) before `aaa`), proving the hardcoded sort.
- [ ] **Step 5: Implement the macro fix.** In `crates/macros/src/lib.rs`, replace the `list` handler body (currently `macros/src/lib.rs:818-845`). Add a per-module `ListQuery` struct to the expansion and use it:
  ```rust
  // (inside the `quote! { pub(crate) mod #module { … } }` expansion, before `list`)
  #[derive(::core::default::Default, ::serde::Deserialize)]
  pub(crate) struct ListQuery {
      pub page: ::core::option::Option<i64>,
      pub per_page: ::core::option::Option<i64>,
      #[serde(alias = "sortBy")]
      pub sort_by: ::core::option::Option<String>,
      pub order: ::core::option::Option<String>,
      pub q: ::core::option::Option<String>,
      pub project: ::core::option::Option<u32>,
      pub parent: ::core::option::Option<u32>,
      pub geotype: ::core::option::Option<String>,
      pub mode: ::core::option::Option<String>,
  }
  // … list(): replace `query: Query<Pagination>` with `query: Query<ListQuery>`:
  pub(crate) async fn list(
      db: actix_web::web::Data<koji_db::KojiDb>,
      query: actix_web::web::Query<ListQuery>,
  ) -> ::core::result::Result<actix_web::HttpResponse, crate::utils::error::ServiceError> {
      let page = query.page.unwrap_or(1).max(1);
      let per_page = query.per_page.unwrap_or(50).clamp(1, 500);
      let args = koji_db::query_args::AdminReqParsed {
          page: (page - 1) as u64,
          per_page: per_page as u64,
          sort_by: query.sort_by.clone().unwrap_or_else(|| "id".to_string()),
          order: query.order.clone().unwrap_or_else(|| "ASC".to_string()),
          q: query.q.clone().unwrap_or_default(),
          geotype: query.geotype.clone(),
          project: query.project,
          mode: query.mode.clone(),
          parent: query.parent,
          geofenceid: ::core::option::Option::None,
          pointsmin: ::core::option::Option::None,
          pointsmax: ::core::option::Option::None,
      };
      let (results, total, _has_next, _has_prev) =
          koji_db::db::#module::Query::paginate(&db.koji, args).await?.into_parts();
      ::core::result::Result::Ok(crate::utils::api_response::ApiResponse::success_paginated(
          results,
          crate::utils::api_response::Meta::build(total as i64, page, per_page),
      ))
  }
  ```
  Keep the `#[utoipa::path]` params block; add `("sortBy" = Option<String>, Query, …), ("order" = …), ("q" = …)` so the public OpenAPI reflects the new query (general-correctness, public surface).
- [ ] **Step 6: Run both, expect PASS.** `cargo test -p macros --test koji_resource 2>&1 | tail -8` (unit guard) and `set -a; source ./.env.test; set +a; cargo test -p koji-service --test internal_geofences_rows macro_list_sorts 2>&1 | tail -10` → `ok`. Also `cargo test -p koji-service resources:: 2>&1 | tail` to confirm the existing macro DTO unit tests still pass.
- [ ] **Step 7: Commit.** `git add crates/macros crates/koji-service/tests/internal_geofences_rows.rs && git commit -m "fix(macros): koji_resource list honors sortBy/order/q + filters (was hardcoded id/ASC)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 6: Mount the `/internal` scope — forward-aliases + row-list + WS, behind the same auth

**Files:**
- Modify: `crates/koji-service/src/internal/mod.rs` (the public `scope()` builder)
- Modify: `crates/koji-service/src/lib.rs` (build `RealtimeHub` into app state; mount `/internal` under `public_validator`)
- Test: `crates/koji-service/tests/internal_forwards.rs` (Create)

**Interfaces:**
- Produces: `pub(crate) fn scope(hub: web::Data<RealtimeHub>) -> actix_web::Scope` mounting:
  - bespoke `GET /internal/geofences` → `geofences::list_rows`;
  - forwarded geofence item routes (`getOne/create/update/delete` + `/publish` + `/golbat-data`) — reuse `public::v2::geofences::scope()`-equivalent handlers, but the GET-list at `""` is the row-list (override);
  - forwarded `projects`/`properties`/`tile-servers` via the macro `scope()`s; `plugins`, `config`, `auth`, `routes`, `nominatim` via their `scope()`s;
  - `GET /internal/realtime` → `realtime::realtime_ws`.
- Consumes: every existing `pub(crate) fn scope()` in `public::v2::*` (geofences/routes/resources/plugins/auth + `config::config`, `nominatim::search_nominatim`), `RealtimeHub`.

- [ ] **Step 1: Failing forward-alias test (auth + parity).** Create `crates/koji-service/tests/internal_forwards.rs`. Two no-DB-needed checks first: (a) unauthenticated `/internal/config` is `401` when `KOJI_SECRET` set; (b) the route registers (not `404`). Use a new `koji_service::test_internal_app(db, hub)` that mirrors prod wiring (auth + `/internal` scope):
  ```rust
  #![allow(clippy::await_holding_lock)]
  use actix_web::test;

  #[actix_web::test]
  async fn internal_config_requires_auth_when_secret_set() {
      unsafe { std::env::set_var("KOJI_SECRET", "topsecret"); }
      let app = test::init_service(koji_service::test_internal_authed_app()).await;
      let req = test::TestRequest::get().uri("/internal/config").to_request();
      let resp = test::call_service(&app, req).await;
      assert_eq!(resp.status().as_u16(), 401, "no bearer → 401");

      let req = test::TestRequest::get().uri("/internal/config")
          .insert_header(("Authorization", "Bearer topsecret")).to_request();
      let resp = test::call_service(&app, req).await;
      assert!(resp.status().is_success(), "valid bearer → config");
      unsafe { std::env::remove_var("KOJI_SECRET"); }
  }

  #[actix_web::test]
  async fn internal_auth_me_is_forwarded() {
      unsafe { std::env::set_var("KOJI_SECRET", ""); }
      let app = test::init_service(koji_service::test_internal_authed_app()).await;
      let req = test::TestRequest::get().uri("/internal/auth/me").to_request();
      let resp = test::call_service(&app, req).await;
      assert!(resp.status().is_success());
      let v: serde_json::Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
      assert_eq!(v["status"], "ok");
      assert!(v["data"]["authenticated"].is_boolean());
  }
  ```
  `test_internal_authed_app()` mounts only the DB-free `/internal` members (`config`, `auth`, `realtime`) wrapped in `public_validator`, with session middleware + a `RealtimeHub`. (Full DB-backed parity — `/internal/geofences/{id}` shape == `/api/v2/geofences/{id}` — gets a DB-gated test in this file too, see Step 6.)
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p koji-service --test internal_forwards 2>&1 | tail -20` → expect `cannot find function \`test_internal_authed_app\``.
- [ ] **Step 3: Implement the `/internal` scope builder.** In `crates/koji-service/src/internal/mod.rs`:
  ```rust
  pub(crate) mod geofences;
  pub(crate) mod realtime;

  use actix_web::web;
  use crate::public::v2;
  use realtime::RealtimeHub;

  /// The `/internal` scope: forward-aliases (same handlers as `/api/v2`) +
  /// the bespoke geofence row-list (overrides only GET-list) + the WS hub.
  /// Mounted behind the SAME `public_validator` auth as `/api/v2`. Not in OpenAPI.
  pub(crate) fn scope() -> actix_web::Scope {
      web::scope("/internal")
          // bespoke row-list overrides GET; the rest of the geofence routes forward.
          .service(web::resource("/geofences").route(web::get().to(geofences::list_rows)))
          .service(v2::geofences::internal_item_scope())  // {id} getOne/patch/delete + publish + golbat-data, NO GET-list
          .service(v2::routes::scope())
          .service(v2::resources::project::scope())
          .service(v2::resources::property::scope())
          .service(v2::resources::tile_server::scope())
          .service(v2::plugins::scope())
          .service(v2::config::config)
          .service(v2::nominatim::search_nominatim)
          .service(v2::auth::scope())
          .service(web::resource("/realtime").route(web::get().to(realtime::realtime_ws)))
  }
  ```
  Add `pub(crate) fn internal_item_scope()` to `public/v2/geofences.rs` — a `scope("/geofences")` identical to `scope()` but **without** the `web::resource("").route(get→list)` (POST stays). This lets `/internal/geofences` GET be the row-list while create/getOne/patch/delete/publish forward. The simplest implementation:
  ```rust
  /// Like `scope()` but the collection GET is omitted (the `/internal` row-list
  /// overrides it); collection POST + all `/{id}` routes forward unchanged.
  pub(crate) fn internal_item_scope() -> actix_web::Scope {
      web::scope("/geofences")
          .service(web::resource("").route(web::post().to(create)))
          .service(web::resource("/{id}/publish").route(web::post().to(publish)))
          .service(web::resource("/{id}/golbat-data").route(web::get().to(crate::public::v2::golbat_data::golbat_data)))
          .service(web::resource("/{id}").route(web::get().to(get_one)).route(web::patch().to(update)).route(web::delete().to(remove)))
  }
  ```
  Make `v2::geofences`, `v2::routes`, `v2::resources`, `v2::plugins`, `v2::config`, `v2::nominatim`, `v2::auth` and their `scope()`/handler fns reachable from `internal` — they are `pub(crate)`, same crate, so they already are; `internal::scope` references `crate::public::v2`.
- [ ] **Step 4: Mount in prod + build the hub.** In `crates/koji-service/src/lib.rs` `start()`:
  - After the `events`/`jobs` setup (~line 280), build `let hub = std::sync::Arc::new(internal::realtime::RealtimeHub::new());`.
  - In the `HttpServer::new` closure, add `.app_data(web::Data::from(hub.clone()))` next to the other `app_data`.
  - After the `/api` scope `.service(...)`, add the `/internal` scope behind the same auth:
    ```rust
    .service(
        internal::scope().wrap(HttpAuthentication::with_fn(auth::public_validator)),
    )
    ```
    (Confirm scope-level `.wrap` order: `actix_web::Scope::wrap` applies the middleware to that scope only — matches how `/v2` wraps its own auth at `lib.rs:344-345`.)
- [ ] **Step 5: Add the test app builders.** In `lib.rs`:
  ```rust
  /// Test surface: the DB-free members of `/internal` wrapped in real auth.
  #[doc(hidden)]
  pub fn test_internal_authed_app() -> actix_web::App<impl actix_web::dev::ServiceFactory<
      actix_web::dev::ServiceRequest, Config=(), Response=actix_web::dev::ServiceResponse,
      Error=actix_web::Error, InitError=()>> {
      let hub = std::sync::Arc::new(internal::realtime::RealtimeHub::new());
      App::new()
          .app_data(web::Data::from(hub))
          .wrap(actix_web_httpauth::middleware::HttpAuthentication::with_fn(auth::public_validator))
          .wrap(actix_session::SessionMiddleware::builder(
              actix_session::storage::CookieSessionStore::default(),
              actix_web::cookie::Key::from(&[0u8;64])).cookie_secure(false).build())
          .service(web::scope("/internal")
              .service(public::v2::config::config)
              .service(public::v2::auth::scope())
              .service(web::resource("/realtime").route(web::get().to(internal::realtime::realtime_ws))))
  }
  ```
  (Note: `wrap` order — auth must be the *outermost* applied so it runs first; in actix `.wrap` applied later = outer. Put auth `.wrap` AFTER session `.wrap` so the session is available to `public_validator`. Verify against the prod ordering at `lib.rs:327-345`.)
- [ ] **Step 6: Add a DB-gated parity test.** Append to `internal_forwards.rs` (mirror `v2_db.rs` helpers): create a geofence via `POST /api/v2/geofences` (or insert), then assert `GET /internal/geofences/{id}` returns the SAME body as `GET /api/v2/geofences/{id}` (both GeoJSON `Feature`). Use `koji_service::test_db_app` extended to also mount `internal::scope()` (no auth, like `test_db_app`). Assert `internal_body == public_body` for the same id.
- [ ] **Step 7: Run it, expect PASS.** No-DB: `cargo test -p koji-service --test internal_forwards internal_config internal_auth 2>&1 | tail -12`. DB: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test internal_forwards 2>&1 | tail -15` (background; cold server/DB). All `ok`.
- [ ] **Step 8: Commit.** `git add crates/koji-service/src && git commit -m "feat(internal): mount /internal scope — forward-aliases + row-list + WS, same auth as /api/v2

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 7: Macro + geofence/route handlers emit resource-mutation events

**Files:**
- Modify: `crates/macros/src/lib.rs` (`create`/`update`/`remove` emit events; add `topic:` parse key)
- Modify: `crates/koji-service/src/public/v2/resources.rs` (add `topic:` to each invocation)
- Modify: `crates/koji-service/src/public/v2/geofences.rs` (emit in create/update/remove)
- Modify: `crates/koji-service/src/public/v2/routes.rs` (emit in create/update/remove)
- Test: `crates/koji-service/tests/internal_realtime_ws.rs` (extend: DB-gated mutation→frame) + `crates/macros/tests/koji_resource.rs` (compile-guard the `topic:` key)

**Interfaces:**
- Produces: every successful `create`/`update`/`delete` (macro + geofence + route) calls `hub.publish(...)` for each `(topic, ServerEvent)` from `topics::created|updated|deleted(name, id, [data])`. The macro handlers gain `hub: web::Data<crate::internal::realtime::RealtimeHub>`.
- Consumes: `RealtimeHub` (app state), `internal::realtime::topics`.
- The macro's resource `{name}` for topics: `project`/`property`/`tileserver` (NOT `tile_server`) — supplied via the new `topic:` invocation key.

- [ ] **Step 1: Failing DB-gated mutation→frame test.** In `internal_realtime_ws.rs` add (DB-gated; boots `actix_web::test::start` with the full `/internal` scope + a shared hub via `test_internal_live_app(db, hub)`):
  ```rust
  #[actix_web::test]
  async fn creating_a_geofence_publishes_resource_event() {
      unsafe { std::env::set_var("KOJI_SECRET", ""); }
      let Some(conn) = db_or_skip().await else { return; };       // KOJI_DB_URL gate helper
      let db = koji_db::build_test_koji_db(conn.clone()).await;
      let hub = koji_service::test_realtime_hub();
      let hub2 = hub.clone();
      let dbc = db.clone();
      let srv = actix_web::test::start(move || koji_service::test_internal_live_app(dbc.clone(), hub2.clone()));
      let ws_url = srv.url("/internal/realtime").replace("http://","ws://");
      let (_r, mut ws) = awc::Client::new().ws(ws_url).connect().await.unwrap();
      ws.send(awc::ws::Message::Text(r#"{"op":"subscribe","topic":"resource/geofence"}"#.into())).await.unwrap();
      tokio::time::sleep(std::time::Duration::from_millis(50)).await;

      // create via the forwarded POST /internal/geofences
      let name = format!("ws-{}", koji_jobs::JobId::new().as_string());
      let body = serde_json::json!({"name":name, "geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}});
      let _ = awc::Client::new().post(srv.url("/internal/geofences")).send_json(&body).await.unwrap();

      let frame = ws.next().await.unwrap().unwrap();
      let awc::ws::Frame::Text(bytes) = frame else { panic!("text") };
      let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
      assert_eq!(v["topic"], "resource/geofence");
      assert_eq!(v["type"], "created");
      assert!(v["payload"]["ids"].as_array().unwrap().len() == 1);
      // cleanup: delete by the created id
      let id = v["payload"]["ids"][0].as_i64().unwrap();
      let _ = conn.execute(sea_orm::Statement::from_sql_and_values(sea_orm::DbBackend::MySql,
          "DELETE FROM geofence WHERE id = ?", [sea_orm::Value::from(id)])).await;
  }
  ```
- [ ] **Step 2: Run it, expect FAIL.** `set -a; source ./.env.test; set +a; cargo test -p koji-service --test internal_realtime_ws creating_a_geofence 2>&1 | tail -15` → FAIL: either `test_internal_live_app` missing, or (once added but no emit) the `ws.next()` hangs/times out → assertion never reached.
- [ ] **Step 3: Emit from the geofence handlers.** In `public/v2/geofences.rs`, add `hub: web::Data<crate::internal::realtime::RealtimeHub>` to `create`/`update`/`remove`; after the successful DB write, publish:
  ```rust
  use crate::internal::realtime::topics;
  // create(): after computing `id`
  for (t, ev) in topics::created("geofence", id as i64) { hub.publish(&t, ev); }
  // update(): after `record` produced; id is the path id
  for (t, ev) in topics::updated("geofence", id as i64, record.clone()) { hub.publish(&t, ev); }
  // remove(): after confirming rows_affected > 0; id is the path id
  for (t, ev) in topics::deleted("geofence", id as i64) { hub.publish(&t, ev); }
  ```
  (`remove` currently consumes `path.into_inner()` inline — bind `let id = path.into_inner();` first.)
- [ ] **Step 4: Emit from the route handlers.** Same edits in `public/v2/routes.rs` `create`/`update`/`remove` with `"route"`.
- [ ] **Step 5: Emit from the macro.** In `crates/macros/src/lib.rs`: (a) add a `topic: LitStr` field to `ResourceDef` parsing (after `seg:`), error if missing like `seg`; (b) in the expansion, add `hub: actix_web::web::Data<crate::internal::realtime::RealtimeHub>` to `create`/`update`/`remove`; (c) publish using a literal topic name `#topic`:
  ```rust
  // create(): after `let id = …`
  for (t, ev) in crate::internal::realtime::topics::created(#topic, id as i64) { hub.publish(&t, ev); }
  // update(): after `record`
  for (t, ev) in crate::internal::realtime::topics::updated(#topic, id as i64, record.clone()) { hub.publish(&t, ev); }
  // remove(): after the rows_affected check; bind id from path first
  for (t, ev) in crate::internal::realtime::topics::deleted(#topic, id as i64) { hub.publish(&t, ev); }
  ```
  In `public/v2/resources.rs` add `topic:` to each invocation: `project → "project"`, `property → "property"`, `tile_server → "tileserver"`.
- [ ] **Step 6: Update test app builders + existing test harnesses to register the hub.** Add `.app_data(web::Data::new(hub))` (or `web::Data::from`) to `test_db_app`, `test_internal_geofences_app`, and add `test_internal_live_app(db, hub)` (full `/internal` scope, no auth, hub in state) + `db_or_skip()` to the WS test file. Every place that builds an App with the macro/geofence/route handlers MUST provide a `RealtimeHub` in app data or the handlers won't construct (missing `web::Data` → 500 at runtime, not compile error — so the parity tests would start failing; add the hub everywhere).
- [ ] **Step 7: Run it, expect PASS.** `set -a; source ./.env.test; set +a; cargo test -p koji-service --test internal_realtime_ws 2>&1 | tail -15` (background). Then the regression sweep: `cargo test -p koji-service --test v2_db 2>&1 | tail` to confirm CRUD still green with the hub added.
- [ ] **Step 8: Commit.** `git add crates/macros crates/koji-service/src crates/koji-service/tests && git commit -m "feat(internal): emit resource-mutation events (created/updated/deleted) from macro + geofence/route handlers

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 8: Job worker emits jobs/{id} + jobs lifecycle events

**Files:**
- Modify: `crates/koji-jobs/src/types.rs` (`JobEventSink` trait; `ProgressHandle` carries + calls the sink)
- Modify: `crates/koji-jobs/src/queue.rs` (`JobQueue` holds `Option<Arc<dyn JobEventSink>>`; `with_event_sink`)
- Modify: `crates/koji-jobs/src/worker.rs` (emit status on claim `running` + terminal outcomes)
- Modify: `crates/koji-service/src/lib.rs` (impl `JobEventSink` for the hub; inject via `with_event_sink`)
- Modify: `crates/koji-service/src/internal/realtime/mod.rs` (impl `koji_jobs::JobEventSink` for `RealtimeHub`)
- Test: `crates/koji-jobs/tests` unit on the sink (no DB) + `crates/koji-service/tests/internal_realtime_ws.rs` (DB-gated job-event via a fake-handler job — optional if calc handler too heavy; prefer the unit test as the gating one).

**Interfaces:**
- Produces:
  - `koji_jobs::JobEventSink` trait: `fn on_job_status(&self, id: &str, status: &str, progress: f32, phase: Option<&str>)` (sync, fire-and-forget) — emits `jobs/{id}` `{type:"status", payload:{id,status,progress,phase?}}` AND `jobs` `{type:"updated", payload:{id,status}}`; and `fn on_job_progress(&self, id, status, progress, phase)` → `jobs/{id}` `{type:"progress", …}`.
  - `JobQueue::with_event_sink(self, sink: Arc<dyn JobEventSink>) -> Self`.
- Consumes: `RealtimeHub::publish`, `topics::{job_topic, jobs_topic}` (the impl lives in koji-service; the trait + payload contract live in koji-jobs).

- [ ] **Step 1: Failing unit test for the sink contract (no DB).** In `crates/koji-jobs/src/types.rs` test module add a recording fake sink + assert the worker-facing builder threads it. Test:
  ```rust
  #[test]
  fn job_event_sink_receives_status() {
      use std::sync::{Arc, Mutex};
      #[derive(Default)] struct Rec(Mutex<Vec<(String,String,f32,Option<String>)>>);
      impl JobEventSink for Rec {
          fn on_job_status(&self, id: &str, status: &str, progress: f32, phase: Option<&str>) {
              self.0.lock().unwrap().push((id.into(), status.into(), progress, phase.map(Into::into)));
          }
          fn on_job_progress(&self, id: &str, status: &str, progress: f32, phase: Option<&str>) {
              self.0.lock().unwrap().push((id.into(), format!("p:{status}"), progress, phase.map(Into::into)));
          }
      }
      let rec = Arc::new(Rec::default());
      let sink: Arc<dyn JobEventSink> = rec.clone();
      sink.on_job_status("01J", "running", 0.0, None);
      sink.on_job_progress("01J", "running", 0.5, Some("clustering"));
      let got = rec.0.lock().unwrap();
      assert_eq!(got[0], ("01J".into(), "running".into(), 0.0, None));
      assert_eq!(got[1].3.as_deref(), Some("clustering"));
  }
  ```
- [ ] **Step 2: Run it, expect FAIL.** `cargo test -p koji-jobs job_event_sink 2>&1 | tail -10` → `cannot find trait \`JobEventSink\``.
- [ ] **Step 3: Define the trait + thread it.** In `crates/koji-jobs/src/types.rs`:
  ```rust
  /// Fire-and-forget sink for job lifecycle events (realtime hub in koji-service
  /// implements it; koji-jobs stays transport-agnostic). All methods are sync +
  /// best-effort — never block or fail the job.
  pub trait JobEventSink: Send + Sync {
      fn on_job_status(&self, id: &str, status: &str, progress: f32, phase: Option<&str>);
      fn on_job_progress(&self, id: &str, status: &str, progress: f32, phase: Option<&str>);
  }
  ```
  Add an optional sink to `ProgressHandle` (it already holds `db` + job id; it also needs the job's `public_id` string + an `Option<Arc<dyn JobEventSink>>`). In `ProgressHandle::set`, after the successful UPDATE, call `if let Some(s) = &self.sink { s.on_job_progress(&self.public_id, "running", progress, phase); }`. Update `ProgressHandle::new` callers accordingly (worker passes the sink + public_id).
- [ ] **Step 4: Carry the sink on `JobQueue`.** In `crates/koji-jobs/src/queue.rs` add `pub event_sink: Option<Arc<dyn JobEventSink>>` to `JobQueue` (default `None` in `new`) + `pub fn with_event_sink(mut self, sink: Arc<dyn JobEventSink>) -> Self { self.event_sink = Some(sink); self }`.
- [ ] **Step 5: Emit from the worker.** In `crates/koji-jobs/src/worker.rs` `run_claimed_job`: after `claimed` is in hand (status now `running`), `if let Some(s) = &queue.event_sink { s.on_job_status(&public_id, "running", 0.0, None); }`. After `persist_outcome`, map the outcome to a status string (`succeeded`/`failed`/`canceled`) and `s.on_job_status(&public_id, status, progress, phase)` (progress 1.0 for succeeded, else last-known/0.0). Build `ProgressHandle::new(queue.db.clone(), claimed.id, public_id.clone(), queue.event_sink.clone())`.
- [ ] **Step 6: Run the unit test, expect PASS.** `cargo test -p koji-jobs 2>&1 | tail -10` → all koji-jobs tests `ok` (the new one + existing).
- [ ] **Step 7: Implement the sink on the hub + inject in prod.** In `crates/koji-service/src/internal/realtime/mod.rs`:
  ```rust
  impl koji_jobs::JobEventSink for RealtimeHub {
      fn on_job_status(&self, id: &str, status: &str, progress: f32, phase: Option<&str>) {
          self.publish(&topics::job_topic(id),
              ServerEvent::new("status", serde_json::json!({"id":id,"status":status,"progress":progress,"phase":phase})));
          self.publish(topics::jobs_topic(),
              ServerEvent::new("updated", serde_json::json!({"id":id,"status":status})));
      }
      fn on_job_progress(&self, id: &str, status: &str, progress: f32, phase: Option<&str>) {
          self.publish(&topics::job_topic(id),
              ServerEvent::new("progress", serde_json::json!({"id":id,"status":status,"progress":progress,"phase":phase})));
      }
  }
  ```
  In `lib.rs` `start()`: after building `hub`, change the `JobQueue` construction to `let jobs = Arc::new(JobQueue::new(databases.koji.clone(), worker_id.clone()).with_event_sink(hub.clone()));` (move the `hub` build *above* the `jobs` build). `hub.clone()` is an `Arc<RealtimeHub>` — coerce to `Arc<dyn JobEventSink>` (the `with_event_sink` param type).
- [ ] **Step 8: Run the full koji-jobs + koji-service build, expect PASS.** `cargo test -p koji-jobs 2>&1 | tail -5` and `cargo build -p koji-service 2>&1 | tail -5` (the prod wiring compiles with the coercion). Optionally the DB-gated job-event WS assertion if a lightweight test handler can be registered.
- [ ] **Step 9: Commit.** `git add crates/koji-jobs crates/koji-service/src && git commit -m "feat(jobs): JobEventSink trait — worker emits jobs/{id} status + jobs updated; hub implements it

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"`

---

### Task 9: Full-suite verification + lint/type gate

**Files:** none (verification only).

**Interfaces:** none.

- [ ] **Step 1: Reconstruct `.env.test` if missing.** `test -f .env.test || cp .env.example .env.test` then ensure `KOJI_DB_URL` points at the live `koji_test` MySQL (per the test-db memory: `mysql` running, `koji_database`/`koji_user`, `cargo run -p migration -- up` applied).
- [ ] **Step 2: Lint + typecheck + tests in parallel (single batch).** Run together:
  - `cargo clippy -p koji-service -p koji-jobs -p macros --all-targets 2>&1 | tail -20` → expect `0 warnings` (or only pre-existing crate-wide allows).
  - `cargo build -p koji-service 2>&1 | tail -5` → clean.
  - `set -a; source ./.env.test; set +a; cargo test -p koji-service -p koji-jobs -p macros 2>&1 | tail -40` → all green (DB-gated tests run; if `KOJI_DB_URL` unset they skip — note which).
  (Background each; >5s.)
- [ ] **Step 3: Confirm no `/internal` leaked into OpenAPI.** `cargo test -p koji-service --test v2_lib_wiring 2>&1 | tail` still passes, and `grep -rn "internal" crates/koji-service/src/utils/openapi.rs` → no matches (the doc must not reference `/internal`).
- [ ] **Step 4: Final commit (only if Step 2 surfaced fixups).** `git add -A && git commit -m "test(internal): full-suite green — /internal forwards, row-list, WS hub, mutation + job events

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"` (skip if nothing changed).

---

## Acceptance (Plan A done when)

- `/internal` mounts behind `public_validator`; unauthenticated requests with a non-empty `KOJI_SECRET` get `401` on every `/internal/*` route incl. the WS handshake.
- `GET /internal/geofences` returns `{status:"ok", data: GeofenceRow[], meta}` honoring `page/per_page/sortBy/order/q` + `project/parent/geotype/mode`, reusing `geofence::Query::paginate`.
- Forward-aliased `/internal/{geofences/{id},projects,properties,tile-servers,plugins,config,auth/*,routes,nominatim}` return byte-identical bodies to their `/api/v2` twins.
- The `koji_resource!` macro `list` honors `sortBy/order/q` + filters (public `/api/v2` fix too).
- `GET /internal/realtime` speaks the exact `ClientFrame`/`ServerFrame` protocol: `subscribe`/`unsubscribe` filter, `ping`→`{op:"pong"}`, subscribed topics receive `{topic,type,payload,meta}`.
- Every create/update/delete (geofence, route, project, property, tileserver, plugins) publishes the exact contract §4 resource events; the job worker publishes `jobs`/`jobs/{id}` status events.
- `cargo test -p koji-service -p koji-jobs -p macros` green with `KOJI_DB_URL` set.
