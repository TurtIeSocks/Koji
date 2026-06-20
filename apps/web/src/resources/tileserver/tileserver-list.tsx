import { DataTable, FilterLiveSearch, UrlField } from '@/components/admin'
import { ListLive } from '@/components/realtime'

export const TileserverList = () => (
  <ListLive>
    <div className="flex w-56 flex-col gap-4">
      <FilterLiveSearch source="q" />
    </div>

    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col label="URL" source="url">
        <UrlField source="url" />
      </DataTable.Col>
    </DataTable>
  </ListLive>
)
