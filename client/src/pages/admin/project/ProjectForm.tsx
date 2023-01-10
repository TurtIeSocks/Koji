import * as React from 'react'
import { SimpleForm, TextInput } from 'react-admin'

export default function ProjectForm() {
  return (
    <SimpleForm>
      <TextInput source="name" fullWidth isRequired />
    </SimpleForm>
  )
}
