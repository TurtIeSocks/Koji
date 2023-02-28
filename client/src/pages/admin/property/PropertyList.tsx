import * as React from 'react'
import {
  BulkDeleteWithUndoButton,
  Datagrid,
  DeleteWithUndoButton,
  EditButton,
  List,
  Pagination,
  TextField,
  TopToolbar,
  CreateButton,
  NumberField,
} from 'react-admin'

function ListActions() {
  return (
    <TopToolbar>
      <CreateButton />
    </TopToolbar>
  )
}

function BulkActions() {
  return <BulkDeleteWithUndoButton resource="property" />
}

export default function PropertyList() {
  return (
    <List
      pagination={<Pagination rowsPerPageOptions={[25, 50, 100]} />}
      title="Properties"
      perPage={25}
      actions={<ListActions />}
      sort={{ field: 'id', order: 'ASC' }}
    >
      <Datagrid rowClick="expand" bulkActionButtons={<BulkActions />}>
        <TextField source="name" />
        <TextField source="category" />
        <TextField source="default_value" />
        <NumberField
          source="geofences.length"
          label="Geofences"
          sortable={false}
        />
        <EditButton />
        <DeleteWithUndoButton />
      </Datagrid>
    </List>
  )
}
