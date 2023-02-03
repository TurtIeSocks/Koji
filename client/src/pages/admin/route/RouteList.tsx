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
  ReferenceField,
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

const defaultSort = { field: 'id', order: 'ASC' }

function BulkActions() {
  return (
    <>
      <BulkDeleteWithUndoButton resource="route" />
      <BulkAssignButton resource="route" />
      <BulkPushToProd resource="route" />
    </>
  )
}

function AreaPagination() {
  return <Pagination rowsPerPageOptions={[25, 50, 100]} />
}

export default function GeofenceList() {
  return (
    <List
      pagination={<AreaPagination />}
      title="Routes"
      perPage={25}
      actions={<ListActions />}
      sort={defaultSort}
    >
      <Datagrid rowClick="expand" bulkActionButtons={<BulkActions />}>
        <TextField source="name" />
        <TextField source="description" />
        <TextField source="mode" />
        <ReferenceField source="geofence_id" reference="geofence" />
        <NumberField source="hops" label="Hops" />
        <EditButton />
        <DeleteWithUndoButton />
        <PushToProd resource="route" />
      </Datagrid>
    </List>
  )
}
