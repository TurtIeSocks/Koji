import { ExportPolygon } from '@components/dialogs/Polygon'
import * as React from 'react'
import {
  BooleanField,
  BulkDeleteWithUndoButton,
  CreateButton,
  Datagrid,
  DeleteWithUndoButton,
  EditButton,
  List,
  NumberField,
  Pagination,
  TextField,
  TopToolbar,
  SearchInput,
} from 'react-admin'
import { BulkAssignButton } from '../actions/AssignProjectFence'
import { BulkExportButton, ExportButton } from '../actions/Export'
import { PushToProd } from '../actions/PushToApi'

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
      <BulkDeleteWithUndoButton resource="project" />
      <BulkAssignButton resource="project" />
      <BulkExportButton resource="project" />
    </>
  )
}

export default function ProjectList() {
  return (
    <>
      <List
        filters={[<SearchInput source="q" alwaysOn />]}
        pagination={<Pagination rowsPerPageOptions={[25, 50, 100]} />}
        title="Projects"
        actions={<ListActions />}
        perPage={25}
        sort={{ field: 'id', order: 'ASC' }}
      >
        <Datagrid rowClick="expand" bulkActionButtons={<BulkActions />}>
          <TextField source="name" />
          <TextField source="description" />
          <BooleanField source="api_endpoint" looseValue />
          <BooleanField source="api_key" looseValue />
          <BooleanField source="scanner" />
          <NumberField source="geofences.length" label="Geofences" />
          <EditButton />
          <DeleteWithUndoButton />
          <PushToProd resource="project" />
          <ExportButton resource="project" />
        </Datagrid>
      </List>
      <ExportPolygon />
    </>
  )
}
