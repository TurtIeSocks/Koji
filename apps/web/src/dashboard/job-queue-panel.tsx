import { useState } from "react";
import { useSubscribe } from "@/components/realtime";
import { Card } from "@/components/ui/card";

interface JobRow {
  id: number;
  status: string;
}

export const JobQueuePanel = () => {
  const [jobs, setJobs] = useState<JobRow[]>([]);

  useSubscribe("jobs", (event) => {
    const payload = event.payload as JobRow | undefined;
    if (!payload?.id) return;
    setJobs((prev) => {
      const next = prev.filter((j) => j.id !== payload.id);
      return [{ id: payload.id, status: payload.status }, ...next].slice(0, 20);
    });
  });

  const running = jobs.filter((j) => j.status === "running").length;

  return (
    <Card className="p-4">
      <h2 className="mb-2 font-semibold">Job queue</h2>
      <p className="text-sm text-muted-foreground">Active: {running}</p>
      <ul className="mt-2 flex flex-col gap-1 text-sm">
        {jobs.map((j) => (
          <li key={j.id} className="flex justify-between">
            <span>Job {j.id}</span>
            <span>{j.status}</span>
          </li>
        ))}
      </ul>
    </Card>
  );
};
