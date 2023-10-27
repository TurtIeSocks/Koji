import { useStatic } from '@hooks/useStatic'
import * as React from 'react'
import { BooleanInput, TextInput } from 'react-admin'

export default function ProjectForm() {
  const { scannerType } = useStatic.getState()

  return (
    <>
      <TextInput source="name" fullWidth isRequired />
      <TextInput source="description" fullWidth />
      <BooleanInput source="scanner" />
      <TextInput
        source="api_endpoint"
        fullWidth
        helperText={
          scannerType === 'unown'
            ? 'Hint! For Unown use this format: http://{host_ip}:{port}/reload'
            : 'Hint! For RDM use this format: http://{host_ip}:{port}/api/set_data?reload_instances=true'
        }
        sx={{ my: 2 }}
      />
      <TextInput
        source="api_key"
        fullWidth
        helperText={
          scannerType === 'unown'
            ? 'Hint! For Unown use this format: {header_name}:{api_key}'
            : 'Hint! For RDM use this format: {username}:{password}'
        }
        sx={{ my: 2 }}
      />
    </>
  )
}
