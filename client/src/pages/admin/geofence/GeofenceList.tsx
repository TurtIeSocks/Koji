import * as React from 'react'
import {
  BulkDeleteWithUndoButton,
  Datagrid,
  EditButton,
  List,
  Pagination,
  TextField,
  TopToolbar,
  CreateButton,
  ReferenceField,
} from 'react-admin'

import { ExportPolygon } from '@components/dialogs/Polygon'
import { GeofenceFilter } from './GeofenceFilter'
import { BulkAssignButton } from '../actions/AssignProjectFence'
import { BulkExportButton } from '../actions/Export'
import { BulkPushToProd, PushToProd } from '../actions/PushToApi'
import { GeofenceExpand } from './GeofenceExpand'
import { BulkAssignFenceButton } from '../actions/AssignParentFence'
import { ExtraMenuActions } from '../actions/Extras'

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
      <BulkAssignFenceButton />
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
        aside={<GeofenceFilter />}
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
          <PushToProd resource="geofence" />
          <ExtraMenuActions resource="geofence" />
        </Datagrid>
      </List>
      <ExportPolygon />
    </>
  )
}
