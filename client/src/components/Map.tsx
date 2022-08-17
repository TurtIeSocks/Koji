import React from 'react'
import { MapContainer, TileLayer } from 'react-leaflet'
import { Box, CssBaseline } from '@mui/material'

import { useStore } from '@hooks/useStore'

import Markers from './markers'
import Interface from './interface'
import Routes from './shapes/Routing'
import QuestPolygon from './shapes/QuestPolygon'
import DrawerIndex from './interface/drawer'
import Main from './interface/styled/Main'

interface Props {
  initial: [number, number]
  tileServer: string
  zoom: number
}

export default function Map({ initial, tileServer, zoom }: Props) {
  const drawer = useStore((s) => s.drawer)
  const setDrawer = useStore((s) => s.setDrawer)

  const [drawerWidth, setDrawerWidth] = React.useState<number>(
    drawer ? 280 : 20,
  )

  React.useEffect(() => {
    setDrawerWidth(drawer ? 280 : 20)
  }, [drawer])

  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />
      <DrawerIndex
        drawer={drawer}
        setDrawer={setDrawer}
        drawerWidth={drawerWidth}
      />
      <Main open={drawer} drawerWidth={drawerWidth}>
        <MapContainer
          key={initial.join('')}
          center={initial || [0, 0]}
          zoom={zoom || 16}
          zoomControl={false}
        >
          <TileLayer
            key={tileServer}
            attribution="Kōji - TurtleSocks"
            url={tileServer}
          />
          <Markers />
          <Interface />
          <Routes />
          <QuestPolygon />
        </MapContainer>
      </Main>
    </Box>
  )
}
