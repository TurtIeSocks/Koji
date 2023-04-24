import * as React from 'react'
import {
  BulkDeleteWithUndoButton,
  Datagrid,
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

import { BulkExportButton } from '../actions/Export'
import { BulkPushToProd, PushToProd } from '../actions/PushToApi'
import { RouteFilter } from './RouteFilter'
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
      <BulkPushToProd resource="route" />
      <BulkDeleteWithUndoButton resource="route" size="small" />
      <BulkExportButton resource="route" />
    </>
  )
}

export default function RouteList() {
  return (
    <>
      <List
        aside={<RouteFilter />}
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
          <PushToProd
            resource="route"
            sx={{ display: { xs: 'none', sm: 'flex' } }}
          />
          <ExtraMenuActions resource="route" />
        </Datagrid>
      </List>
      <ExportPolygon />
    </>
  )
}
