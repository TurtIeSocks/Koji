import React from 'react'
import { MapContainer, TileLayer } from 'react-leaflet'
import { Box, CssBaseline } from '@mui/material'

import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'

import Markers from './markers'
import Interface from './interface'
import Routes from './shapes/Routing'
import QuestPolygon from './shapes/QuestPolygon'
import DrawerIndex from './interface/drawer'
import Main from './interface/styled/Main'
import Drawing from './shapes/Drawing'

export default function Map() {
  const drawer = useStore((s) => s.drawer)
  const { location, zoom } = useStore.getState()

  const tileServer = useStatic((s) => s.tileServer)

  const [drawerWidth, setDrawerWidth] = React.useState<number>(
    drawer ? 280 : 20,
  )

  React.useEffect(() => {
    setDrawerWidth(drawer ? 280 : 20)
  }, [drawer])

  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />
      <DrawerIndex drawerWidth={drawerWidth} />
      <Main open={drawer} drawerWidth={drawerWidth}>
        <MapContainer center={location} zoom={zoom} zoomControl={false}>
          <TileLayer
            key={tileServer}
            attribution="Kōji - TurtleSocks"
            url={tileServer}
          />
          <Markers />
          <Interface />
          <Routes />
          <QuestPolygon />
          <Drawing />
        </MapContainer>
      </Main>
    </Box>
  )
}
