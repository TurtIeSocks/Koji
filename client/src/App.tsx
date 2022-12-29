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
    errorElement: <ErrorPage error="500" />,
  },
  {
    path: '/map',
    element: <Map />,
    errorElement: <ErrorPage error="500" />,
  },
  {
    path: '/admin/*',
    element: <AdminPanel />,
    errorElement: <ErrorPage error="500" />,
  },
  {
    path: '*',
    element: <ErrorPage />,
    errorElement: <ErrorPage error="500" />,
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
