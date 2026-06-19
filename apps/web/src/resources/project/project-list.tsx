import {
  DataTable,
  BooleanField,
  FilterLiveSearch,
  FunctionField,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";

export const ProjectList = () => (
  <ListLive
    aside={
      <div className="flex w-56 flex-col gap-4">
        <FilterLiveSearch source="q" />
      </div>
    }
  >
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="golbat" label="Golbat Sync">
        <BooleanField source="golbat" />
      </DataTable.Col>
      <DataTable.Col source="geofences" label="Geofences">
        <FunctionField
          source="geofences"
          render={(record: any) =>
            Array.isArray(record?.geofences) ? record.geofences.length : 0
          }
        />
      </DataTable.Col>
    </DataTable>
  </ListLive>
);
