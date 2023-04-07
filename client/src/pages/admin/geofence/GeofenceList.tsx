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
  SearchInput,
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
          <TextField source="mode" />
          <TextField source="geo_type" />
          <NumberField
            source="projects.length"
            label="Projects"
            sortable={false}
          />
          <NumberField
            source="properties.length"
            label="Properties"
            sortable={false}
          />
          <NumberField source="routes.length" label="Routes" sortable={false} />
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
