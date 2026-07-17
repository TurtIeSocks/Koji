import { useState } from "react";
import { useNotify, useRecordContext } from "ra-core";
import { Button } from "@/components/ui/button";
import { testWebhook } from "@api";

export const WebhookTestButton = () => {
  const record = useRecordContext();
  const notify = useNotify();
  const [loading, setLoading] = useState(false);

  const onTest = async () => {
    if (!record) return;
    setLoading(true);
    try {
      const data = await testWebhook(record.id);
      if (data.delivered) {
        notify(`Delivered ✓ (${data.upstream_status})`, { type: "success" });
      } else {
        notify(`Delivery failed: ${data.error ?? "unknown"}`, { type: "warning" });
      }
    } catch (e) {
      notify(`Test failed: ${e instanceof Error ? e.message : String(e)}`, { type: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Button type="button" variant="outline" onClick={onTest} disabled={loading}>
      {loading ? "Testing…" : "Test"}
    </Button>
  );
};
