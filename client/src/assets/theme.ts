import { LinkProps } from '@mui/material/Link'
import {
  createTheme,
  responsiveFontSizes,
  type Theme,
} from '@mui/material/styles'
import LinkBehavior from '@components/styled/LinkBehavior'

export default function create(mode: Theme['palette']['mode']) {
  return responsiveFontSizes(
    createTheme({
      palette: {
        mode,
        // primary: {
        //   main: 'rgba(191, 22, 233)',
        // },
        // secondary: {
        //   main: 'rgba(208, 80, 0, 1)',
        // },
      },
      typography: {
        h1: {
          fontSize: '10rem',
        },
      },
      components: {
        MuiLink: {
          defaultProps: {
            component: LinkBehavior,
          } as LinkProps,
        },
        MuiButtonBase: {
          defaultProps: {
            LinkComponent: LinkBehavior,
          },
        },
        MuiGrid2: {
          defaultProps: {
            alignItems: 'center',
            justifyContent: 'center',
            textAlign: 'center',
          },
        },
        MuiPaper: {
          defaultProps: {
            elevation: 0,
            square: true,
          },
        },
      },
    }),
  )
}
