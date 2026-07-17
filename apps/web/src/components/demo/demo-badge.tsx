import { useState } from "react";
import { resetDemo } from "@api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

// `resetDemo` is trivial in the live build (`@api`'s live surface exports a
// no-op — see src/api/live/config.ts), so no dynamic-import gymnastics are
// needed here; the `if (!__DEMO__)` guard below is enough for esbuild/terser
// to dead-code-eliminate `DemoBadgeInner` (and its "Demo"/"Reset demo"
// strings) from the live bundle. Split into a guard + inner component so the
// guard's early return never sits above a hook call (rules-of-hooks).
function DemoBadge() {
  if (!__DEMO__) return null;
  return <DemoBadgeInner />;
}

/** "DEMO" badge + "Reset demo" button for the app-bar toolbar. Reset wipes and
 *  reseeds the demo world (`resetDemo`, pure — no reload), then reloads the
 *  page so every in-memory store/hook picks up the fresh world. */
function DemoBadgeInner() {
  const [resetting, setResetting] = useState(false);

  async function handleReset() {
    setResetting(true);
    try {
      await resetDemo();
      window.location.reload();
    } catch (err) {
      console.error("Failed to reset the demo world:", err);
      setResetting(false);
    }
  }

  return (
    <div className="flex items-center gap-2">
      <Badge className="uppercase tracking-wide" variant="secondary">
        Demo
      </Badge>
      <Button disabled={resetting} onClick={handleReset} size="sm" variant="outline">
        {resetting ? "Resetting…" : "Reset demo"}
      </Button>
    </div>
  );
}

export { DemoBadge };
