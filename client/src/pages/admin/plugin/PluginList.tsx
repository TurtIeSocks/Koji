import * as React from 'react'
import {
  BooleanField,
  Datagrid,
  EditButton,
  List,
  TextField,
} from 'react-admin'

export default function PluginList() {
  return (
    <List title="Plugins" sort={{ field: 'id', order: 'ASC' }}>
      <Datagrid rowClick="edit" bulkActionButtons={false}>
        <TextField source="name" />
        <TextField source="kind" />
        <BooleanField source="enabled" />
        <TextField source="version" />
        <EditButton />
      </Datagrid>
    </List>
  )
}
