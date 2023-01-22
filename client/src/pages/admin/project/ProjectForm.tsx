import * as React from 'react'
import { BooleanInput, TextInput } from 'react-admin'

export default function ProjectForm() {
  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="api_endpoint" fullWidth />
      <TextInput source="api_key" fullWidth />
      <BooleanInput source="scanner" />
    </>
  )
}
