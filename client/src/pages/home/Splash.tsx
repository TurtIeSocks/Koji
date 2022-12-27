import * as React from 'react'
import { Link } from 'react-router-dom'
import Paper from '@mui/material/Paper'
import Grid from '@mui/material/Grid' // https://github.com/mui/material-ui/issues/35643
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
    <Grid
      container
      item
      xs={12}
      md={6}
      component={Link}
      justifyContent="center"
      alignItems="center"
      to={to}
      sx={{
        height: { xs: '50vh', md: '100vh' },
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
    </Grid>
  )
}
export default function Splash() {
  const darkMode = usePersist((s) => s.darkMode)

  return (
    <Grid container component={Paper} height="100vh" square>
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
    </Grid>
  )
}
