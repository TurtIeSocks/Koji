import * as React from 'react'
import { Admin, RaThemeOptions, Resource, defaultTheme } from 'react-admin'
import { useTheme } from '@mui/material'
import Architecture from '@mui/icons-material/Architecture'
import AccountTree from '@mui/icons-material/AccountTree'

import { dataProvider } from './dataProvider'
import Layout from './Layout'
import GeofenceList from './geofence/GeofenceList'
import GeofenceShow from './geofence/GeofenceShow'
import GeofenceEdit from './geofence/GeofenceEdit'
import ProjectList from './project/ProjectList'
import ProjectEdit from './project/ProjectEdit'
import ProjectShow from './project/ProjectShow'
import ProjectCreate from './project/ProjectCreate'

export default function AdminPanel() {
  const theme = useTheme()

  return (
    <Admin
      basename="/admin"
      title="Kōji Admin"
      dataProvider={dataProvider}
      theme={{
        ...defaultTheme,
        ...(theme as RaThemeOptions),
      }}
      layout={Layout}
    >
      <Resource
        name="geofence"
        icon={Architecture}
        list={GeofenceList}
        edit={GeofenceEdit}
        show={GeofenceShow}
      />
      <Resource
        name="project"
        icon={AccountTree}
        list={ProjectList}
        edit={ProjectEdit}
        show={ProjectShow}
        create={ProjectCreate}
      />
    </Admin>
  )
}
