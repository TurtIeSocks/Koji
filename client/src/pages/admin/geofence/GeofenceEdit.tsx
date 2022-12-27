import * as React from 'react'
import { Edit } from 'react-admin'

// import { KojiGeofence } from '@assets/types'
import { ClientGeofence } from '@assets/types'

import GeofenceForm from './GeofenceForm'

const transformPayload = (geofence: ClientGeofence) => {
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
