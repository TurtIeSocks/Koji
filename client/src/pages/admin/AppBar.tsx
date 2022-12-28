import * as React from 'react'
import { Box, IconButton, Typography } from '@mui/material'
import { AppBar as BaseAppBar } from 'react-admin'
import Home from '@mui/icons-material/Home'

import ThemeToggle from '@components/ThemeToggle'

export default function AppBar() {
  return (
    <BaseAppBar>
      <Box flex="1">
        <Typography variant="h6" id="react-admin-title" />
      </Box>
      <IconButton href="/" sx={{ color: 'white' }}>
        <Home fontSize="medium" />
      </IconButton>
      <ThemeToggle />
    </BaseAppBar>
  )
}
