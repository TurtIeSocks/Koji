import * as React from 'react'
import { Create, SimpleForm, useNotify, useRedirect } from 'react-admin'
import { Typography } from '@mui/material'
import { KojiRoute } from '@assets/types'

import RouteForm from './RouteForm'

const transformPayload = (route: KojiRoute) => {
  const { geometry: raw, ...rest } = route
  const geometry = typeof raw === 'string' ? JSON.parse(raw) : raw
  return {
    ...rest,
    id: 0,
    geometry,
  }
}

export default function RouteCreate() {
  const notify = useNotify()
  const redirect = useRedirect()

  const onSuccess = () => {
    notify('Route created successfully')
    redirect('list', 'route')
  }

  return (
    <Create
      title="Create a Route"
      mutationOptions={{ onSuccess }}
      transform={transformPayload}
    >
      <SimpleForm>
        <Typography>Create One</Typography>
        <RouteForm />
      </SimpleForm>
    </Create>
  )
}
