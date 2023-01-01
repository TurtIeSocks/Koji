import * as React from 'react'
import Paper from '@mui/material/Paper'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import GradientText from '@components/GradientText'
import { Box } from '@mui/material'
import ThemeToggle from '@components/ThemeToggle'
import { usePersist } from '@hooks/usePersist'

interface Props {
  children: React.ReactNode
  darkMode: boolean
  to: string
}

function GridLink({ darkMode, to, children }: Props) {
  return (
    <Grid2
      container
      xs={12}
      sm={6}
      component="a"
      href={to}
      justifyContent="center"
      alignItems="center"
      sx={{
        height: { xs: '50vh', sm: '100vh' },
        transition: '0.5s ease',
        '&:hover': {
          background: darkMode
            ? 'rgba(255, 255, 255, 0.2)'
            : 'rgba(0, 0, 0, 0.2)',
          backgroundSize: 'cover',
          backgroundRepeat: 'no-repeat',
          backgroundPosition: 'center',
        },
        textDecoration: 'none',
      }}
    >
      {children}
    </Grid2>
  )
}
export default function Home() {
  const darkMode = usePersist((s) => s.darkMode)

  return (
    <Grid2 container component={Paper} height="100vh" square>
      <GridLink darkMode={darkMode} to="/map">
        <GradientText className="map" variant="h1">
          Map
        </GradientText>
      </GridLink>
      <GridLink darkMode={darkMode} to="/admin">
        <GradientText className="admin" variant="h1">
          Admin
        </GradientText>
      </GridLink>
      <Box sx={{ position: 'absolute', top: 10, right: 10 }}>
        <ThemeToggle />
      </Box>
    </Grid2>
  )
}
