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
  SearchInput,
} from 'react-admin'
import { ExportPolygon } from '@components/dialogs/Polygon'

import { BulkExportButton, ExportButton } from '../actions/Export'
import { BulkPushToProd, PushToProd } from '../actions/PushToApi'

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
      <BulkDeleteWithUndoButton resource="route" />
      <BulkPushToProd resource="route" />
      <BulkExportButton resource="route" />
    </>
  )
}

export default function RouteList() {
  return (
    <>
      <List
        filters={[<SearchInput source="q" alwaysOn />]}
        pagination={<Pagination rowsPerPageOptions={[25, 50, 100]} />}
        title="Routes"
        perPage={25}
        actions={<ListActions />}
        sort={{ field: 'id', order: 'ASC' }}
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
