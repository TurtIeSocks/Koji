import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { Button, Typography } from '@mui/material'
import { useStatic } from '@hooks/useStatic'
import { useShapes } from '@hooks/useShapes'

import Notification from './Base'

export default function CombinedPolyNotif() {
  const combinedPolyMode = useStatic((s) => s.combinePolyMode)
  const combined = useShapes((s) => s.combined)

  return (
    <Notification
      CollapseProps={{
        in:
          combinedPolyMode &&
          Object.values(combined).filter(Boolean).length > 1,
      }}
      AlertProps={{
        severity: 'success',
      }}
      IconButtonProps={{
        sx: { color: 'white' },
        onClick: () => useStatic.getState().setStatic('combinePolyMode', false),
      }}
      title="Combine Polygon Mode"
    >
      <Grid2 container width="50vw" justifyContent="flex-start">
        <Grid2 xs={6}>
          <Typography color="white" align="left">
            {Object.values(combined).filter(Boolean).length} polygons set to
            combine
          </Typography>
        </Grid2>
        <Grid2 xs={6}>
          <Button
            size="small"
            onClick={() => {
              useShapes.getState().setters.combine()
              return useStatic.getState().setStatic('combinePolyMode', false)
            }}
            variant="contained"
            color="primary"
          >
            Combine
          </Button>
        </Grid2>
      </Grid2>
    </Notification>
  )
}
