import React from 'react'
import { Box, CssBaseline } from '@mui/material'
import { MapContainer, Pane, TileLayer } from 'react-leaflet'

import { usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'

import DrawerIndex from '../../components/drawer'
import Main from '../../components/styled/Main'
import Markers from './markers'
import Interface from './interface'
import ErrorBoundary from '../../components/ErrorBoundary'
import Features from './markers/Features'

export default function Map() {
  const drawer = usePersist((s) => s.drawer)
  const { location, zoom } = usePersist.getState()
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
          <Features />
        </MapContainer>
      </Main>
    </Box>
  )
}
