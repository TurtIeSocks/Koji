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
  SearchInput,
  ReferenceField,
} from 'react-admin'

import { ExportPolygon } from '@components/dialogs/Polygon'

import { BulkAssignButton } from '../actions/AssignButton'
import { BulkExportButton, ExportButton } from '../actions/Export'
import { BulkPushToProd, PushToProd } from '../actions/PushToApi'
import { GeofenceExpand } from './GeofenceExpand'

function ListActions() {
  return (
    <TopToolbar>
      <CreateButton />
    </TopToolbar>
  )
}

function BulkActions() {
  return (
    <>
      <BulkDeleteWithUndoButton resource="geofence" />
      <BulkAssignButton resource="geofence" />
      <BulkPushToProd resource="geofence" />
      <BulkExportButton resource="geofence" />
    </>
  )
}

export default function GeofenceList() {
  return (
    <>
      <List
        filters={[<SearchInput source="q" alwaysOn />]}
        pagination={<Pagination rowsPerPageOptions={[25, 50, 100]} />}
        title="Geofences"
        perPage={25}
        actions={<ListActions />}
        sort={{ field: 'id', order: 'ASC' }}
      >
        <Datagrid
          rowClick="expand"
          bulkActionButtons={<BulkActions />}
          expand={GeofenceExpand}
        >
          <TextField source="name" />
          <ReferenceField source="parent" reference="geofence" />
          <TextField source="mode" />
          <TextField source="geo_type" />
          <EditButton />
          <DeleteWithUndoButton />
          <PushToProd resource="geofence" />
          <ExportButton resource="geofence" />
        </Datagrid>
      </List>
      <ExportPolygon />
    </>
  )
}
