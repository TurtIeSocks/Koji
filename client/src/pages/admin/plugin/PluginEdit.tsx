import * as React from 'react'
import { BooleanInput, Edit, SimpleForm, TextInput } from 'react-admin'

export default function PluginEdit() {
  return (
    <Edit mutationMode="pessimistic">
      <SimpleForm>
        {/* read-only manifest fields (disk-owned) */}
        <TextInput source="entrypoint" disabled />
        <TextInput source="interpreter" disabled />
        <TextInput source="protocol" disabled />
        {/* editable overlay */}
        <BooleanInput source="enabled" />
        <TextInput
          source="args_default"
          multiline
          fullWidth
          parse={(v: string) => {
            try {
              return JSON.parse(v)
            } catch {
              return v
            }
          }}
          format={(v: unknown) =>
            typeof v === 'string' ? v : JSON.stringify(v ?? null)
          }
        />
        <TextInput source="description" fullWidth />
      </SimpleForm>
    </Edit>
  )
}
