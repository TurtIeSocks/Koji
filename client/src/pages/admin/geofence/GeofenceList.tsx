import * as React from 'react'
import {
  // CreateButton,
  Datagrid,
  DeleteWithUndoButton,
  EditButton,
  List,
  NumberField,
  Pagination,
  TextField,
  // TopToolbar,
} from 'react-admin'

// function ListActions() {
//   return (
//     <TopToolbar>
//       <CreateButton />
//     </TopToolbar>
//   )
// }

const defaultSort = { field: 'id', order: 'ASC' }

function AreaPagination() {
  return <Pagination rowsPerPageOptions={[25, 50, 100]} />
}

export default function GeofenceList() {
  return (
    <List
      pagination={<AreaPagination />}
      title="Geofences"
      // actions={<ListActions />}
      perPage={25}
      sort={defaultSort}
    >
      <Datagrid rowClick="show" bulkActionButtons={false}>
        <TextField source="name" />
        <NumberField source="related.length" label="Projects" />
        <EditButton />
        <DeleteWithUndoButton />
      </Datagrid>
    </List>
  )
}
