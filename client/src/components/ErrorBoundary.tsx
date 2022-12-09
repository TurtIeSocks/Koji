/* eslint-disable react/destructuring-assignment */
import React, { Component } from 'react'
import {
  Alert,
  AlertTitle,
  Button,
  Collapse,
  IconButton,
  Stack,
  Typography,
} from '@mui/material'
import CloseIcon from '@mui/icons-material/Close'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { Refresh } from '@mui/icons-material'

type Props = {
  children: React.ReactNode
}

type State = {
  hasError: boolean
  message: string
  errorCount: number
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
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
        <Collapse
          in={this.state.hasError}
          sx={{
            position: 'absolute',
            bottom: 0,
            width: '66%',
            mx: 'auto',
            left: 0,
            right: 0,
            transition: '0.50s ease-in-out',
          }}
        >
          <Stack sx={{ width: '100%' }} spacing={2}>
            <Alert
              variant="filled"
              severity="error"
              action={
                <IconButton
                  aria-label="close"
                  color="inherit"
                  size="small"
                  onClick={() => this.setState({ hasError: false })}
                >
                  <CloseIcon fontSize="inherit" />
                </IconButton>
              }
              sx={{ mb: 2, zIndex: 10000 }}
            >
              <AlertTitle>
                <strong>Kōji encountered an error!</strong>
              </AlertTitle>
              <Typography>{this.state.message}</Typography>
            </Alert>
          </Stack>
        </Collapse>
        {this.props.children || null}
      </>
    )
  }
}
