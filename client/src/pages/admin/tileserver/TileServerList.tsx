import * as React from 'react'
import {
  BulkDeleteWithUndoButton,
  CreateButton,
  Datagrid,
  DeleteWithUndoButton,
  EditButton,
  List,
  Pagination,
  TextField,
  TopToolbar,
} from 'react-admin'

function ListActions() {
  return (
    <TopToolbar>
      <CreateButton />
    </TopToolbar>
  )
}

function BulkActions() {
  return <BulkDeleteWithUndoButton resource="tileserver" />
}

export default function TileServerList() {
  return (
    <List
      pagination={<Pagination rowsPerPageOptions={[25, 50, 100]} />}
      title="TileServers"
      actions={<ListActions />}
      perPage={25}
      sort={{ field: 'id', order: 'ASC' }}
    >
      <Datagrid rowClick="expand" bulkActionButtons={<BulkActions />}>
        <TextField source="name" />
        <TextField source="url" />
        <EditButton />
        <DeleteWithUndoButton />
      </Datagrid>
    </List>
  )
}
