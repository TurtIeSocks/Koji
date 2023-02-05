import * as React from 'react'
import { Create, SimpleForm, useNotify, useRedirect } from 'react-admin'
import { Typography } from '@mui/material'
import { AdminGeofence } from '@assets/types'

import PropertyForm from './PropertyForm'

const transformPayload = async (geofence: AdminGeofence) => {
  return {
    id: 0,
    name: geofence.name,
    mode: geofence.mode,
    area: {
      ...JSON.parse(geofence.area.toString()),
      properties: Object.fromEntries(
        geofence.properties.map((p) => [p.key, p.value]),
      ),
    },
  }
}

export default function PropertyCreate() {
  const notify = useNotify()
  const redirect = useRedirect()

  const onSuccess = () => {
    notify('Property created successfully')
    redirect('list', 'property')
  }

  return (
    <Create
      title="Create a Property"
      mutationOptions={{ onSuccess }}
      // transform={transformPayload}
    >
      <SimpleForm>
        <Typography>Create One</Typography>
        <PropertyForm />
      </SimpleForm>
    </Create>
  )
}
