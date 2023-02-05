import * as React from 'react'
import {
  BulkDeleteWithUndoButton,
  Datagrid,
  DeleteWithUndoButton,
  EditButton,
  List,
  NumberField,
  Pagination,
  TextField,
  TopToolbar,
  CreateButton,
} from 'react-admin'
import { BulkAssignButton } from '../actions/bulk/AssignButton'
import { BulkPushToProd, PushToProd } from '../actions/bulk/PushToApi'

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
        <EditButton />
        <DeleteWithUndoButton />
      </Datagrid>
    </List>
  )
}
