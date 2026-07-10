import { Create, SimpleForm, TextInput } from "@/components/admin";
import { DeckMap } from "@/components/deck";
import { useStartCenter } from "@/components/leaflet/use-start-center";
import { useWatch } from "react-hook-form";
import { required } from "ra-core";

function TileserverPreview() {
  const url = useWatch({ name: "url" }) as string | undefined;
  const tileUrl = url && url.trim().length > 0 ? url : null;
  const [lat, lon] = useStartCenter();
  if (!tileUrl) {
    return (
      <div className="flex h-40 items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground">
        Enter a tile URL above to preview the map.
      </div>
    );
  }
  return (
    <DeckMap
      layers={[]}
      tileUrl={tileUrl}
      height={300}
      initialViewState={{ longitude: lon, latitude: lat, zoom: 10 }}
    />
  );
}

export const TileserverFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="url" label="Tile URL" validate={required()} />
    <TileserverPreview />
  </>
);

export const TileserverCreate = () => (
  <Create>
    <SimpleForm>
      <TileserverFormFields />
    </SimpleForm>
  </Create>
);
