import * as React from 'react'
import { Box, IconButton, Typography } from '@mui/material'
import { AppBar as BaseAppBar } from 'react-admin'
import Home from '@mui/icons-material/Home'
import Map from '@mui/icons-material/Map'
import InfoIcon from '@mui/icons-material/Info'

import ThemeToggle from '@components/ThemeToggle'

export default function AppBar() {
  return (
    <BaseAppBar>
      <Box flex="1">
        <Typography variant="h6" id="react-admin-title" />
      </Box>
      <IconButton
        href="https://koji.vercel.app/"
        LinkComponent="a"
        target="_blank"
        referrerPolicy="no-referrer"
        sx={{ color: 'white' }}
      >
        <InfoIcon />
      </IconButton>
      <IconButton href="/" sx={{ color: 'white' }}>
        <Home fontSize="medium" />
      </IconButton>
      <IconButton href="/map" sx={{ color: 'white' }}>
        <Map fontSize="medium" />
      </IconButton>
      <ThemeToggle />
    </BaseAppBar>
  )
}
