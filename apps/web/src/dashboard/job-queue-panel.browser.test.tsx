import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import {
  realtimeDataProvider,
  fakeTransport,
  inMemoryLockProvider,
} from "@/components/realtime";
import { baseDataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { JobQueuePanel } from "@/dashboard/job-queue-panel";

describe("JobQueuePanel", () => {
  it("appends a job on a jobs event", async () => {
    const transport = fakeTransport();
    const dp = realtimeDataProvider(baseDataProvider, transport, {
      locks: inMemoryLockProvider(),
    });
    const screen = render(
      <AdminContext dataProvider={dp} authProvider={authProvider}>
        <JobQueuePanel />
      </AdminContext>,
    );
    await transport.publish("jobs", {
      type: "updated",
      payload: { id: 42, status: "running" },
    });
    await expect.element(screen.getByText(/42/)).toBeVisible();
    await expect.element(screen.getByText(/running/i)).toBeVisible();
  });
});
