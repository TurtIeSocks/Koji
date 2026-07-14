import { TabbedForm } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { RouteMap } from "@/components/deck";
import { RouteMetaFields } from "./route-create";
import type { EditProps } from "@/components/admin/views/edit";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <RouteMetaFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <RouteMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </EditLive>
);
