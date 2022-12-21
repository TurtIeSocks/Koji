import React from 'react'
import { Box, CssBaseline } from '@mui/material'
import { MapContainer, Pane, TileLayer } from 'react-leaflet'

import { useStore } from '@hooks/useStore'
import { useStatic } from '@hooks/useStatic'
import Drawing from '@components/markers/Drawing'

import DrawerIndex from './interface/drawer'
import Main from './interface/styled/Main'
import Markers from './markers'
import Interface from './interface'
import PolygonPopup from './popups/Polygon'
import ErrorBoundary from './ErrorBoundary'
import Vectors from './markers/Vectors'

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
        <MapContainer
          key="map"
          center={location}
          zoom={zoom}
          zoomControl={false}
        >
          <TileLayer
            key={tileServer}
            attribution="<a href='https://github.com/TurtIeSocks/Koji' noreferrer='true' target='_blank'>Kōji - TurtleSocks</a>"
            url={tileServer}
          />
          <Pane name="circles" style={{ zIndex: 600 }} />
          <Pane name="lines" style={{ zIndex: 550 }} />
          <Pane name="polygons" style={{ zIndex: 500 }} />
          <Markers />
          <Interface />
          <PolygonPopup />
          <Drawing key="drawing" />
          <Vectors />
        </MapContainer>
      </Main>
    </Box>
  )
}
