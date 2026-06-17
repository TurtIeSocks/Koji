/* eslint-disable react/destructuring-assignment */
import React, { Component } from 'react'
import { Button, Typography } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import Refresh from '@mui/icons-material/Refresh'
import Notification from './notifications/Base'

export default class ErrorBoundary extends Component<
  {
    children: React.ReactNode
  },
  {
    hasError: boolean
    message: string
    errorCount: number
  }
> {
  constructor(props: { children: React.ReactNode }) {
    super(props)
    this.state = { hasError: false, message: '', errorCount: 0 }
  }

  componentDidCatch(error: Error) {
    this.setState((prev) => ({
      hasError: true,
      message: error?.message || '',
      errorCount: prev.errorCount + 1,
    }))
  }

  render() {
    return this.state.errorCount > 5 ? (
      <Grid2
        container
        alignItems="center"
        justifyContent="center"
        sx={{ height: '100vh', width: '100vw', textAlign: 'center' }}
      >
        <Grid2 xs={12}>
          <Typography variant="h3" align="center">
            Kōji encountered an error!
          </Typography>
          <Typography variant="h6" align="center">
            {this.state.message}
          </Typography>
          <br />
          <br />
          <Button
            onClick={() => window.location.reload()}
            variant="contained"
            color="primary"
            startIcon={<Refresh />}
          >
            Refresh
          </Button>
        </Grid2>
      </Grid2>
    ) : (
      <>
        <Notification
          CollapseProps={{ in: this.state.hasError }}
          title="Kōji encountered an error and has attempted to recover"
          IconButtonProps={{
            onClick: () => this.setState({ hasError: false }),
          }}
          AlertProps={{ severity: 'error' }}
        >
          <Typography>{this.state.message}</Typography>
        </Notification>
        {this.props.children || null}
      </>
    )
  }
}
