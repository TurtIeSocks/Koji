/* eslint-disable react/destructuring-assignment */
import React, { Component } from 'react'
import {
  Alert,
  AlertTitle,
  Collapse,
  IconButton,
  Stack,
  Typography,
} from '@mui/material'
import CloseIcon from '@mui/icons-material/Close'

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
    return (
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
