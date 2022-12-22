import * as React from 'react'
import { Typography, IconButton, styled, useTheme } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import ChevronLeft from '@mui/icons-material/ChevronLeft'
import Brightness7Icon from '@mui/icons-material/Brightness7'
import Brightness4Icon from '@mui/icons-material/Brightness4'

import { useStore } from '@hooks/useStore'

const DrawerHeaderRaw = styled(Grid2)(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'flex-end',
  padding: theme.spacing(0, 1),
  ...theme.mixins.toolbar,
}))

interface Props {
  children: React.ReactNode
}

export default function DrawerHeader({ children }: Props) {
  const darkMode = useStore((s) => s.darkMode)
  const setStore = useStore((s) => s.setStore)

  const theme = useTheme()

  return (
    <DrawerHeaderRaw container>
      <Grid2 xs={2}>
        <IconButton
          sx={{ ml: 1 }}
          onClick={() => setStore('darkMode', !darkMode)}
          color="inherit"
        >
          {theme.palette.mode === 'dark' ? (
            <Brightness7Icon />
          ) : (
            <Brightness4Icon />
          )}
        </IconButton>
      </Grid2>
      <Grid2 xs={8}>
        <Typography variant="h5" align="center">
          {children}
        </Typography>
      </Grid2>
      <Grid2 xs={2}>
        <IconButton onClick={() => setStore('drawer', false)}>
          <ChevronLeft />
        </IconButton>
      </Grid2>
    </DrawerHeaderRaw>
  )
}
