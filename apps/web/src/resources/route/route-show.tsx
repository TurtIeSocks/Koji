import { RecordField, ReferenceField, SelectField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
// Direct submodule import (not the `@/components/deck` barrel): the barrel
// re-exports the whole deck.gl/maplibre map stack, which a jsdom unit test
// can't mount. Importing this file directly keeps the mock target
// (`vi.mock("@/components/deck/route-show-map", ...)` in route-show.test.tsx)
// exact, without dragging in every other deck submodule as a side effect.
import { RouteShowMap } from "@/components/deck/route-show-map";
import { ROUTE_MODES } from "@/lib/constants";
import type { ShowProps } from "@/components/admin/views/show";

// Name is already shown via the breadcrumb + "Route <name>" title chrome
// above this component — no need to repeat it in the body.
export const RouteShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <RecordField source="mode">
          <SelectField source="mode" choices={[...ROUTE_MODES]} />
        </RecordField>
        <RecordField source="description" />
        <RecordField source="geofence_id" label="Geofence">
          <ReferenceField source="geofence_id" reference="geofence" empty="—" />
        </RecordField>
      </div>
      <RouteShowMap />
    </div>
  </ShowLive>
);
