import * as React from 'react'
import { Typography, IconButton, styled } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import ChevronLeft from '@mui/icons-material/ChevronLeft'

import { usePersist } from '@hooks/usePersist'
import ThemeToggle from '@components/ThemeToggle'

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
  const setStore = usePersist((s) => s.setStore)

  return (
    <DrawerHeaderRaw container>
      <Grid2 xs={2}>
        <ThemeToggle />
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
