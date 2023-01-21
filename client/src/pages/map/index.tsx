import React from 'react'
import { Box } from '@mui/material'
import { Pane } from 'react-leaflet'

import { usePersist } from '@hooks/usePersist'

import Map from '@components/Map'
import ErrorBoundary from '@components/ErrorBoundary'
import DrawerIndex from '@components/drawer'
import Main from '@components/styled/Main'
import Loading from '@components/Loading'
import NetworkAlert from '@components/notifications/NetworkStatus'

import Markers from './markers'
import Interface from './interface'

import {
  LineStrings,
  MultiLineStrings,
  MultiPoints,
  Points,
  Polygons,
} from './markers/Vectors'

export default function MapWrapper() {
  const drawer = usePersist((s) => s.drawer)
  const menuItem = usePersist((s) => s.menuItem)

  const drawerWidth = menuItem === 'Geojson' ? 500 : 325

  return (
    <Box sx={{ display: 'flex' }}>
      <Loading />
      <ErrorBoundary>
        <DrawerIndex drawerWidth={drawerWidth} />
      </ErrorBoundary>
      <Main open={drawer} drawerWidth={drawerWidth}>
        <Map
          style={{
            position: 'absolute',
            maxWidth: '100% !important',
            maxHeight: '100% !important',
            margin: 'auto',
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
          }}
        >
          <Pane name="circles" style={{ zIndex: 503 }} />
          <Pane name="lines" style={{ zIndex: 502 }} />
          <Pane name="arrows" style={{ zIndex: 501 }} />
          <Pane name="polygons" style={{ zIndex: 500 }} />
          <Markers category="pokestop" />
          <Markers category="spawnpoint" />
          <Markers category="gym" />
          <Interface />
          <Points />
          <MultiPoints />
          <LineStrings />
          <MultiLineStrings />
          <Polygons />
        </Map>
        <NetworkAlert />
      </Main>
    </Box>
  )
}
