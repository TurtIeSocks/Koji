import * as React from 'react'
import { Admin, Resource, defaultTheme } from 'react-admin'
import { useTheme } from '@mui/material'
import Map from '@mui/icons-material/Map'

import { dataProvider } from './dataProvider'
import Layout from './Layout'
import GeofenceList from './geofence/GeofenceList'
import GeofenceShow from './geofence/GeofenceShow'
import GeofenceEdit from './geofence/GeofenceEdit'

export default function AdminPanel() {
  const theme = useTheme()

  return (
    <Admin
      basename="/admin"
      title="Kōji Admin"
      dataProvider={dataProvider}
      theme={{
        ...defaultTheme,
        ...theme,
        components: defaultTheme.components,
      }}
      layout={Layout}
    >
      <Resource
        name="geofence"
        icon={Map}
        list={GeofenceList}
        edit={GeofenceEdit}
        show={GeofenceShow}
      />
    </Admin>
  )
}
