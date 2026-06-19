import type { ReactNode } from "react";
import { useResourceContext, useRecordContext } from "shadmin-core";
import { useQueryClient } from "@tanstack/react-query";
import { Edit, type EditProps } from "@/components/admin/views/edit";
import { useSubscribeToRecord } from "./hooks/use-subscribe-to-record";
import { useOnReconnect } from "./hooks/use-on-reconnect";

export function EditLive(props: EditProps & { children?: ReactNode }) {
  return (
    <Edit {...props}>
      <EditLiveSubscription />
      {props.children}
    </Edit>
  );
}

function EditLiveSubscription() {
  const resource = useResourceContext();
  const record = useRecordContext();
  const queryClient = useQueryClient();
  const invalidate = () => {
    if (!resource) return;
    queryClient.invalidateQueries({
      predicate: (q) => q.queryKey[0] === resource,
    });
  };
  useSubscribeToRecord(resource ?? "", record?.id, invalidate, {
    enabled: !!resource && record?.id != null,
  });
  useOnReconnect(invalidate);
  return null;
}
