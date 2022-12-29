import * as React from 'react'
import {
  Datagrid,
  DeleteWithUndoButton,
  EditButton,
  List,
  NumberField,
  Pagination,
  TextField,
} from 'react-admin'
import { BulkAssignButton } from '../actions/bulk/AssignButton'

const defaultSort = { field: 'id', order: 'ASC' }

function AreaPagination() {
  return <Pagination rowsPerPageOptions={[25, 50, 100]} />
}

export default function GeofenceList() {
  return (
    <List
      pagination={<AreaPagination />}
      title="Geofences"
      perPage={25}
      sort={defaultSort}
    >
      <Datagrid
        rowClick="expand"
        bulkActionButtons={<BulkAssignButton resource="geofence" />}
      >
        <TextField source="name" />
        <NumberField source="related.length" label="Projects" />
        <EditButton />
        <DeleteWithUndoButton />
      </Datagrid>
    </List>
  )
}
