import * as React from 'react'
import { Admin, RaThemeOptions, Resource, defaultTheme } from 'react-admin'
import { useTheme } from '@mui/material'
import Architecture from '@mui/icons-material/Architecture'
import AccountTree from '@mui/icons-material/AccountTree'
import Route from '@mui/icons-material/Route'
import ListAlt from '@mui/icons-material/ListAlt'
import Map from '@mui/icons-material/Map'

import NetworkAlert from '@components/notifications/NetworkStatus'
import { getFullCache } from '@services/fetches'

import { dataProvider } from './dataProvider'
import Layout from './Layout'

import GeofenceList from './geofence/GeofenceList'
import GeofenceShow from './geofence/GeofenceShow'
import GeofenceEdit from './geofence/GeofenceEdit'
import GeofenceCreate from './geofence/GeofenceCreate'

import ProjectList from './project/ProjectList'
import ProjectEdit from './project/ProjectEdit'
import ProjectShow from './project/ProjectShow'
import ProjectCreate from './project/ProjectCreate'

import RouteList from './route/RouteList'
import RouteEdit from './route/RouteEdit'
import RouteShow from './route/RouteShow'
import RouteCreate from './route/RouteCreate'

import PropertyList from './property/PropertyList'
import PropertyEdit from './property/PropertyEdit'
import PropertyShow from './property/PropertyShow'
import PropertyCreate from './property/PropertyCreate'

import TileServerList from './tileserver/TileServerList'
import TileServerEdit from './tileserver/TileServerEdit'
import TileServerShow from './tileserver/TileServerShow'
import TileServerCreate from './tileserver/TileServerCreate'

export default function AdminPanel() {
  const theme = useTheme()

  React.useEffect(() => {
    getFullCache()
  }, [])

  return (
    <>
      <Admin
        basename="/admin"
        title="Kōji Admin"
        dataProvider={dataProvider}
        disableTelemetry
        theme={{
          ...defaultTheme,
          ...(theme as RaThemeOptions),
        }}
        layout={Layout}
      >
        <Resource
          name="project"
          icon={AccountTree}
          list={ProjectList}
          edit={ProjectEdit}
          show={ProjectShow}
          create={ProjectCreate}
          recordRepresentation={(record) => record.name || ''}
        />
        <Resource
          name="geofence"
          icon={Architecture}
          list={GeofenceList}
          edit={GeofenceEdit}
          show={GeofenceShow}
          create={GeofenceCreate}
          recordRepresentation={(record) => record.name || ''}
        />
        <Resource
          name="route"
          icon={Route}
          list={RouteList}
          edit={RouteEdit}
          show={RouteShow}
          create={RouteCreate}
          recordRepresentation={(record) => record.name || ''}
        />
        <Resource
          name="property"
          icon={ListAlt}
          list={PropertyList}
          edit={PropertyEdit}
          show={PropertyShow}
          create={PropertyCreate}
          recordRepresentation={(record) => record.name || ''}
        />
        <Resource
          name="tileserver"
          icon={Map}
          list={TileServerList}
          edit={TileServerEdit}
          show={TileServerShow}
          create={TileServerCreate}
          recordRepresentation={(record) => record.name || ''}
        />
      </Admin>
      <NetworkAlert />
    </>
  )
}
