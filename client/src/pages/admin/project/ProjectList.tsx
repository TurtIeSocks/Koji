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
} from 'react-admin'
import { BulkAssignButton } from '../actions/bulk/AssignButton'

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
    </>
  )
}

const defaultSort = { field: 'id', order: 'ASC' }

function AreaPagination() {
  return <Pagination rowsPerPageOptions={[25, 50, 100]} />
}

export default function ProjectList() {
  return (
    <List
      pagination={<AreaPagination />}
      title="Projects"
      actions={<ListActions />}
      perPage={25}
      sort={defaultSort}
    >
      <Datagrid rowClick="expand" bulkActionButtons={<BulkActions />}>
        <TextField source="name" />
        <BooleanField source="scanner" />
        <BooleanField source="api_endpoint" looseValue />
        <BooleanField source="api_key" looseValue />
        <BooleanField source="scanner" />
        <NumberField source="related.length" label="Geofences" />
        <EditButton />
        <DeleteWithUndoButton />
      </Datagrid>
    </List>
  )
}
