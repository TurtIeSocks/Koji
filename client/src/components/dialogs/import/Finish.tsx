import Button from '@mui/material/Button'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import * as React from 'react'

import { useShapes } from '@hooks/useShapes'
import type { FeatureCollection } from '@assets/types'
import SaveToKoji from '@components/buttons/SaveToKoji'
import DownloadBtn from '@components/buttons/Download'

interface Props {
  filtered: FeatureCollection
  reset: () => void
}

export default function FinishStep({ filtered, reset }: Props) {
  const withInternalProps = {
    ...filtered,
    features: filtered.features.map((feat) => ({
      ...feat,
      properties: {
        ...feat.properties,
        __name: feat.properties?.name,
        __mode: feat.properties?.mode,
        __projects: feat.properties?.projects,
        __geofence_id: feat.properties?.geofence_id,
        __parent: feat.properties?.parent,
        name: undefined,
        mode: undefined,
        projects: undefined,
        geofence_id: undefined,
        parent: undefined,
      },
    })),
  }

  return (
    <Grid2
      container
      minHeight="50vh"
      width="100%"
      direction="column"
      justifyContent="space-around"
    >
      <Grid2>
        <SaveToKoji fc={withInternalProps} variant="outlined" color="success" />
      </Grid2>
      <Grid2>
        <Button
          variant="outlined"
          color="success"
          onClick={() => {
            useShapes.getState().setters.add(withInternalProps.features)
            reset()
          }}
        >
          Send to Map for Further Editing
        </Button>
      </Grid2>
      <Grid2>
        <DownloadBtn data={filtered} />
      </Grid2>
    </Grid2>
  )
}
