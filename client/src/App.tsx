import * as React from 'react'
import { CssBaseline, ThemeProvider } from '@mui/material'
import { createBrowserRouter, RouterProvider } from 'react-router-dom'

import createTheme from '@assets/theme'
import { usePersist } from '@hooks/usePersist'

import Home from '@pages/home'
import Map from '@pages/map'
import AdminPanel from '@pages/admin'
import ErrorPage from '@pages/Error'

const router = createBrowserRouter([
  {
    path: '/',
    element: <Home />,
  },
  {
    path: '/map',
    element: <Map />,
  },
  {
    path: '/admin/*',
    element: <AdminPanel />,
  },
  {
    path: '*',
    element: <ErrorPage />,
  },
])

export default function App() {
  const darkMode = usePersist((s) => s.darkMode)

  const theme = React.useMemo(() => {
    const newTheme = createTheme(darkMode ? 'dark' : 'light')
    document.body.style.backgroundColor = newTheme.palette.background.paper
    return newTheme
  }, [darkMode])

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <RouterProvider router={router} />
    </ThemeProvider>
  )
}
