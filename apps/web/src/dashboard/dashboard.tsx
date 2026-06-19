import { Count } from "@/components/admin";
import { Card } from "@/components/ui/card";
import { JobQueuePanel } from "@/dashboard/job-queue-panel";

const CountCard = ({ resource, label }: { resource: string; label: string }) => (
  <Card className="p-4">
    <p className="text-sm text-muted-foreground">{label}</p>
    <p className="text-2xl font-semibold">
      <Count resource={resource} />
    </p>
  </Card>
);

export function Dashboard() {
  return (
    <div className="flex flex-col gap-4 p-4">
      <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
        <CountCard resource="geofence" label="Geofences" />
      </div>
      <JobQueuePanel />
    </div>
  );
}
