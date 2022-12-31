import * as React from 'react'
import {
  ReferenceArrayInput,
  SelectArrayInput,
  SimpleForm,
  TextInput,
} from 'react-admin'

export default function ProjectForm() {
  return (
    <SimpleForm>
      <TextInput source="name" fullWidth isRequired />
      <ReferenceArrayInput
        source="related"
        reference="geofence"
        label="Geofences"
      >
        <SelectArrayInput optionText="name" />
      </ReferenceArrayInput>
    </SimpleForm>
  )
}
