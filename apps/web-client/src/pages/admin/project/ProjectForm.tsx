import * as React from 'react'
import { BooleanInput, TextInput } from 'react-admin'

export default function ProjectForm() {
  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="description" fullWidth />
      <BooleanInput source="golbat" />
      <TextInput
        source="api_endpoint"
        fullWidth
        helperText="Hint! For Unown use this format: http://{host_ip}:{port}/reload"
        sx={{ my: 2 }}
      />
      <TextInput
        source="api_key"
        fullWidth
        helperText="Hint! For Unown use this format: {header_name}:{api_key}"
        sx={{ my: 2 }}
      />
    </>
  )
}
