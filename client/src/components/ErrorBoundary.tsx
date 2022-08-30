/* eslint-disable react/destructuring-assignment */
import React, { Component } from 'react'
import { Grid, Typography, Button } from '@mui/material'
import { Refresh } from '@mui/icons-material'

type Props = {
  children: React.ReactNode
}

type State = {
  hasError: boolean
  message: string
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, message: '' }
  }

  componentDidCatch(error: Error) {
    this.setState({
      hasError: true,
      message: error?.message || '',
    })
  }

  render() {
    return this.state.hasError ? (
      <Grid
        container
        alignItems="center"
        justifyContent="center"
        sx={{ height: '100vh', width: '100vw', textAlign: 'center' }}
      >
        <Grid item xs={12}>
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
        </Grid>
      </Grid>
    ) : (
      this.props.children || null
    )
  }
}
