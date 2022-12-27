import * as React from 'react'
import { useTheme } from '@mui/material'

interface Props {
  children: React.ReactNode
  className: 'koji' | 'map' | 'admin'
}

export default function GradientText({ children, className }: Props) {
  const theme = useTheme()
  return (
    <div className="section-process">
      <div className="section-container">
        <div className="process-steps-container container-medium with-padding">
          <div className={`process-step-container ${className}`}>
            <div className="process-step-title-container">
              <h1
                className="process-step-title"
                style={{ color: theme.palette.background.paper }}
              >
                {children}
              </h1>
              <div className="process-step-title-overlay">{children}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
