import { ColorField, FunctionField, TextField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";

export const PropertyShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <TextField source="category" />
      <FunctionField
        source="default_value"
        label="Default Value"
        render={(record: any) =>
          record?.category === "color" ? (
            <ColorField source="default_value" record={record} />
          ) : (
            <TextField source="default_value" record={record} />
          )
        }
      />
    </div>
  </ShowLive>
);
