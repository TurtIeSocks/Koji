import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { TileserverFormFields } from "./tileserver-create";

export const TileserverEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <TileserverFormFields />
    </SimpleForm>
  </EditLive>
);
