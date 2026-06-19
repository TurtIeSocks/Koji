import { TextField, UrlField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { BaseMap } from "@/components/leaflet";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "shadmin-core";
import { DEFAULT_TILE_URL } from "@/lib/constants";

function TileserverMapPreview() {
  const record = useRecordContext<{ url?: string }>();
  const tileUrl = record?.url ?? DEFAULT_TILE_URL;
  return <BaseMap tileUrl={tileUrl} height={300} />;
}

export const TileserverShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <UrlField source="url" label="Tile URL" />
      <TileserverMapPreview />
    </div>
  </ShowLive>
);
