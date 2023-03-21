import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { Box, Button, Typography } from '@mui/material'
import Map from '@mui/icons-material/Map'
import Admin from '@mui/icons-material/AdminPanelSettings'
import Convert from '@mui/icons-material/PrecisionManufacturing'
import { MapContainer, TileLayer } from 'react-leaflet'
import { shallow } from 'zustand/shallow'

import ThemeToggle from '@components/ThemeToggle'
import { usePersist } from '@hooks/usePersist'
import { ATTRIBUTION } from '@assets/constants'

export default function Home() {
  const [darkMode, location, zoom, tileServer] = usePersist(
    (s) => [s.darkMode, s.location, s.zoom, s.tileServer],
    shallow,
  )

  return (
    <MapContainer
      center={location}
      zoom={zoom}
      style={{ height: '100vh', width: '100vw' }}
      zoomControl={false}
      scrollWheelZoom={false}
      dragging={false}
      ref={(ref) => {
        if (ref) {
          ref.attributionControl.setPrefix(ATTRIBUTION)
        }
      }}
    >
      <TileLayer
        key={darkMode.toString()}
        url={
          darkMode
            ? 'https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png'
            : tileServer ||
              'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png'
        }
      />
      <Box sx={{ position: 'absolute', top: 10, right: 10, zIndex: 1000 }}>
        <ThemeToggle />
      </Box>
      <Grid2
        sx={{
          position: 'absolute',
          top: 0,
          right: 0,
          bottom: 0,
          left: 0,
          zIndex: 999,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <Box
          sx={{
            padding: 2,
            background: 'rgba(125, 0, 208, 0.25)',
            borderRadius: '16px',
            boxShadow: '0 10px 30px rgba(0, 0, 0, 0.6)',
            backdropFilter: 'blur(6.9px)',
            WebkitBackdropFilter: 'blur(6.9px)',
          }}
        >
          <Grid2
            container
            height="100%"
            zIndex={10000}
            flexDirection={{ xs: 'column', sm: 'row' }}
          >
            {(['Map', 'Admin', 'Convert'] as const).map((page) => {
              const Icon = {
                Map,
                Admin,
                Convert,
              }[page]
              return (
                <Grid2
                  key={page}
                  component={Button}
                  href={`/${page.toLowerCase()}`}
                  color="primary"
                  sx={{
                    m: 3,
                    height: 100,
                    width: 100,
                    background: 'rgba(125, 0, 208, 0.4)',
                    borderRadius: '16px',
                    boxShadow: '0 10px 30px rgba(0, 0, 0, 0.6)',
                    backdropFilter: 'blur(6.9px)',
                    WebkitBackdropFilter: 'blur(6.9px)',
                    display: 'flex',
                    flexDirection: 'column',
                    '&:hover': {
                      background: 'rgba(208, 80, 0, 0.75)',
                    },
                    textDecoration: 'none',
                  }}
                >
                  <Icon fontSize="large" sx={{ my: 1, color: 'white' }} />
                  <Typography fontWeight="bold" sx={{ color: 'white' }}>
                    {page}
                  </Typography>
                </Grid2>
              )
            })}
          </Grid2>
        </Box>
      </Grid2>
    </MapContainer>
  )
}
