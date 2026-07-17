import { describe, expect, it } from "vitest";
import { demoBus } from "./realtime";

describe("demoBus", () => {
  it("delivers published events to topic subscribers", async () => {
    const got: unknown[] = [];
    const un = demoBus.subscribe("jobs/1", (e) => got.push(e));
    await demoBus.publish("jobs/1", { type: "status", payload: { status: "running" } });
    await new Promise((r) => queueMicrotask(() => r(null)));
    expect(got).toHaveLength(1);
    un();
    await demoBus.publish("jobs/1", { type: "status", payload: { status: "succeeded" } });
    await new Promise((r) => queueMicrotask(() => r(null)));
    expect(got).toHaveLength(1);
  });

  it("reports connected immediately", () => {
    let status = "";
    demoBus.onStatusChange?.((s) => {
      status = String(s);
    });
    expect(status).toBe("connected");
  });

  it("stamps the topic onto the delivered event frame", async () => {
    let frame: { topic?: string; type?: string } | null = null;
    const un = demoBus.subscribe("jobs/42", (e) => {
      frame = e as { topic: string; type: string };
    });
    await demoBus.publish("jobs/42", { type: "status", payload: { status: "queued" } });
    await new Promise((r) => queueMicrotask(() => r(null)));
    un();
    expect(frame).not.toBeNull();
    expect(frame!.topic).toBe("jobs/42");
    expect(frame!.type).toBe("status");
  });

  it("isolates topics — a subscriber only hears its own topic", async () => {
    const a: unknown[] = [];
    const b: unknown[] = [];
    const ua = demoBus.subscribe("jobs/a", (e) => a.push(e));
    const ub = demoBus.subscribe("jobs/b", (e) => b.push(e));
    await demoBus.publish("jobs/a", { type: "status", payload: {} });
    await new Promise((r) => queueMicrotask(() => r(null)));
    ua();
    ub();
    expect(a).toHaveLength(1);
    expect(b).toHaveLength(0);
  });
});
