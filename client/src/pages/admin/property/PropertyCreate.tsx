import * as React from 'react'
import { Create, SimpleForm, useNotify, useRedirect } from 'react-admin'
import { Typography } from '@mui/material'

import PropertyForm from './PropertyForm'

export default function PropertyCreate() {
  const notify = useNotify()
  const redirect = useRedirect()

  const onSuccess = () => {
    notify('Property created successfully')
    redirect('list', 'property')
  }

  return (
    <Create title="Create a Property" mutationOptions={{ onSuccess }}>
      <SimpleForm>
        <Typography>Create One</Typography>
        <PropertyForm create />
      </SimpleForm>
    </Create>
  )
}
