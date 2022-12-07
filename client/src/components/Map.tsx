import React from 'react'
import { Box, CssBaseline } from '@mui/material'
import { MapContainer, TileLayer } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { useStatic } from '@hooks/useStatic'

import DrawerIndex from './interface/drawer'
import Main from './interface/styled/Main'
import Markers from './markers'
import Interface from './interface'
import Routes from './shapes/Routing'
import Drawing from './shapes/Drawing'
import PolygonPopup from './popups/Polygon'
import ErrorBoundary from './ErrorBoundary'

export default function Map() {
  const drawer = useStore((s) => s.drawer)
  const { location, zoom } = useStore.getState()
  const tileServer = useStatic((s) => s.tileServer)

  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />
      <ErrorBoundary>
        <DrawerIndex />
      </ErrorBoundary>
      <Main open={drawer} drawerWidth={280}>
        <MapContainer center={location} zoom={zoom} zoomControl={false}>
          <TileLayer
            key={tileServer}
            attribution="<a href='https://github.com/TurtIeSocks/Koji' noreferrer='true' target='_blank'>Kōji - TurtleSocks</a>"
            url={tileServer}
          />
          <Markers />
          <Interface />
          <Routes />
          <Drawing />
          <PolygonPopup />
        </MapContainer>
      </Main>
    </Box>
  )
}
