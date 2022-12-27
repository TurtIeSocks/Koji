import * as React from 'react'
import { Typography, type TypographyProps, useTheme } from '@mui/material'

interface Props {
  children: React.ReactNode
  className: 'koji' | 'map' | 'admin' | 'error'
  variant?: TypographyProps['variant']
}

export default function GradientText({ children, className, variant }: Props) {
  const theme = useTheme()
  return (
    <div className="section-process">
      <div className="section-container">
        <div className="process-steps-container container-medium with-padding">
          <div className={`process-step-container ${className}`}>
            <div className="process-step-title-container">
              <Typography
                variant={variant}
                className="process-step-title"
                style={{ color: theme.palette.background.paper }}
              >
                {children}
              </Typography>
              <Typography
                variant={variant}
                className="process-step-title-overlay"
              >
                {children}
              </Typography>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
