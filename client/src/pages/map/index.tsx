import React from 'react'
import { Box } from '@mui/material'
import { Pane } from 'react-leaflet'

import { usePersist } from '@hooks/usePersist'

import Map from '@components/Map'
import ErrorBoundary from '@components/ErrorBoundary'
import DrawerIndex from '@components/drawer'
import Main from '@components/styled/Main'

import Markers from './markers'
import Interface from './interface'
import Features from './markers/Features'

export default function MapWrapper() {
  const drawer = usePersist((s) => s.drawer)

  return (
    <Box sx={{ display: 'flex' }}>
      <ErrorBoundary>
        <DrawerIndex />
      </ErrorBoundary>
      <Main open={drawer} drawerWidth={450}>
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
          <Pane name="circles" style={{ zIndex: 600 }} />
          <Pane name="lines" style={{ zIndex: 550 }} />
          <Pane name="polygons" style={{ zIndex: 500 }} />
          <Markers />
          <Interface />
          <Features />
        </Map>
      </Main>
    </Box>
  )
}
