import { TextField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";

export const PropertyShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <TextField source="category" />
      <TextField source="default_value" label="Default Value" />
    </div>
  </ShowLive>
);
