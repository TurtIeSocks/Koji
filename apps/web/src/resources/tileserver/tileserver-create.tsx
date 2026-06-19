import { Create, SimpleForm, TextInput } from "@/components/admin";
import { BaseMap } from "@/components/leaflet";
import { useWatch } from "react-hook-form";
import { required } from "ra-core";

function TileserverPreview() {
  const url = useWatch({ name: "url" }) as string | undefined;
  const tileUrl = url && url.trim().length > 0 ? url : null;
  if (!tileUrl) {
    return (
      <div className="flex h-40 items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground">
        Enter a tile URL above to preview the map.
      </div>
    );
  }
  return <BaseMap tileUrl={tileUrl} height={300} />;
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
