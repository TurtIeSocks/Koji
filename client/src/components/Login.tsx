import * as React from 'react'
import Box from '@mui/material/Box'
import { Button, Collapse, TextField, Typography } from '@mui/material'
import { useStatic } from '@hooks/useStatic'

export default function Login() {
  const [password, setPassword] = React.useState<string>('')
  const [error, setError] = React.useState<string>('')

  const setStatic = useStatic((s) => s.setStatic)

  const onSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const res = await fetch('/api/login', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ password }),
    })
    if (res.status === 200) {
      setStatic('loggedIn', true)
    } else {
      setError('Wrong Password')
    }
  }

  return (
    <Box
      display="flex"
      alignItems="center"
      justifyContent="center"
      height="100vh"
      width="100%"
      bgcolor="#333333"
      flexDirection="column"
    >
      <form
        autoComplete="off"
        onSubmit={onSubmit}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <TextField
          name="password"
          label="Password"
          variant="outlined"
          value={password}
          onChange={(e) => {
            setError('')
            setPassword(e.target.value)
          }}
          error={!!error}
        />
        <Button color={error ? 'error' : 'primary'} type="submit">
          Login
        </Button>
      </form>
      <Collapse in={!!error}>
        <Typography color="error" sx={{ mt: 2 }}>
          {error}
        </Typography>
      </Collapse>
    </Box>
  )
}
