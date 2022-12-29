import * as React from 'react'
import { Button, Paper } from '@mui/material'
import { Link } from 'react-router-dom'

import GradientText from '@components/GradientText'

export default function ErrorPage({ error = '404' }: { error?: string }) {
  return (
    <Paper
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexDirection: 'column',
        height: '100vh',
      }}
    >
      <GradientText className="error" variant="h1">
        {error}
      </GradientText>
      <Button component={Link} to="/">
        Back
      </Button>
    </Paper>
  )
}
