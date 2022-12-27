import * as React from 'react'
import { Admin, Resource } from 'react-admin'
import Map from '@mui/icons-material/Map'

import { dataProvider } from './dataProvider'
import GeofenceList from './geofence/GeofenceList'

export default function AdminPanel() {
  return (
    <Admin basename="/admin" dataProvider={dataProvider}>
      <Resource
        name="geofence"
        icon={Map}
        list={GeofenceList}
        // edit={AccountEdit}
        // create={AccountCreate}
        // show={AccountShow}
      />
    </Admin>
  )
}
