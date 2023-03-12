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
import { ExportPolygon } from '@components/dialogs/Polygon'

import { BulkAssignButton } from '../actions/AssignButton'
import { BulkExportButton, ExportButton } from '../actions/Export'
import { BulkPushToProd, PushToProd } from '../actions/PushToApi'

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
      <BulkExportButton resource="route" />
    </>
  )
}

function AreaPagination() {
  return <Pagination rowsPerPageOptions={[25, 50, 100]} />
}

export default function GeofenceList() {
  return (
    <>
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
          <NumberField source="hops" label="Hops" sortable={false} />
          <EditButton />
          <DeleteWithUndoButton />
          <PushToProd resource="route" />
          <ExportButton resource="route" />
        </Datagrid>
      </List>
      <ExportPolygon />
    </>
  )
}
