import * as React from 'react'
import { Typography, IconButton, styled } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { ChevronLeft } from '@mui/icons-material'

const DrawerHeaderRaw = styled(Grid2)(() => ({
  display: 'flex',
  alignItems: 'center',
  minHeight: 56,
  justifyContent: 'flex-end',
}))

interface Props {
  setDrawer: (drawer: boolean) => void
  children: React.ReactNode
}

export default function DrawerHeader({ setDrawer, children }: Props) {
  if (!setDrawer) return <DrawerHeaderRaw />

  return (
    <DrawerHeaderRaw container>
      <Grid2 xs={10}>
        <Typography variant="h5" align="center">
          {children}
        </Typography>
      </Grid2>
      <Grid2 xs={2}>
        <IconButton onClick={() => setDrawer(false)}>
          <ChevronLeft />
        </IconButton>
      </Grid2>
    </DrawerHeaderRaw>
  )
}
