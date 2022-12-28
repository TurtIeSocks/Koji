import * as React from 'react'
import { Edit } from 'react-admin'

// import { KojiGeofence } from '@assets/types'
import { ClientGeofence } from '@assets/types'

import GeofenceForm from './GeofenceForm'

const transformPayload = async (geofence: ClientGeofence) => {
  if (Array.isArray(geofence.related)) {
    await fetch(`/internal/admin/geofence_project/geofence/${geofence.id}`, {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(geofence.related),
    })
  }
  return {
    ...geofence,
    area: {
      ...geofence.area,
      properties: Object.fromEntries(
        geofence.properties.map((p) => [p.key, p.value]),
      ),
    },
  }
}

export default function GeofenceEdit() {
  return (
    <Edit mutationMode="pessimistic" transform={transformPayload}>
      <GeofenceForm />
    </Edit>
  )
}
