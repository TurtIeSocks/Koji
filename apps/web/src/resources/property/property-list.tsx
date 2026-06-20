import {
  ColorField,
  DataTable,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  FunctionField,
  TextField,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { PROPERTY_CATEGORIES } from "@/lib/constants";

export const PropertyList = () => (
  <ListLive
    aside={
      <div className="flex w-56 flex-col gap-4">
        <FilterLiveSearch source="q" />
        <FilterList label="Category">
          {PROPERTY_CATEGORIES.map((c) => (
            <FilterListItem key={c.id} label={c.name} value={{ category: c.id }} />
          ))}
        </FilterList>
      </div>
    }
  >
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="category" />
      <DataTable.Col source="default_value" label="Default">
        <FunctionField
          source="default_value"
          render={(record: any) =>
            record?.category === "color" ? (
              <ColorField source="default_value" record={record} />
            ) : (
              <TextField source="default_value" record={record} />
            )
          }
        />
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
