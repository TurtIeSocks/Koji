import * as React from 'react'
import { Create, SimpleForm, useNotify, useRedirect } from 'react-admin'
import { Typography } from '@mui/material'
import { KojiRoute } from '@assets/types'

import RouteForm from './RouteForm'

const transformPayload = (route: KojiRoute) => {
  return {
    id: 0,
    name: route.name,
    mode: route.mode,
    geometry:
      typeof route.geometry === 'string'
        ? JSON.parse(route.geometry)
        : route.geometry,
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
