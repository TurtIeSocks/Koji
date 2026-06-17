import { styled } from '@mui/material'

export default styled('main', {
  shouldForwardProp: (prop) => prop !== 'open' && prop !== 'drawerWidth',
})<{
  open?: boolean
  drawerWidth: number
}>(({ theme, open, drawerWidth }) => ({
  flexGrow: 1,
  padding: theme.spacing(3),
  transition: theme.transitions.create('margin', {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.leavingScreen,
  }),
  marginLeft: `-${drawerWidth}px`,
  ...(open && {
    transition: theme.transitions.create('margin', {
      easing: theme.transitions.easing.easeOut,
      duration: theme.transitions.duration.enteringScreen,
    }),
    marginLeft: 0,
  }),
}))
