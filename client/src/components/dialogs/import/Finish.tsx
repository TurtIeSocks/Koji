import Button from '@mui/material/Button'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import * as React from 'react'

import { useShapes } from '@hooks/useShapes'
import type { FeatureCollection } from '@assets/types'
import SaveToKoji from '@components/buttons/SaveToKoji'

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
        name: undefined,
        mode: undefined,
        projects: undefined,
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
        <SaveToKoji
          fc={JSON.stringify(withInternalProps)}
          variant="outlined"
          color="success"
        />
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
        <Button
          variant="outlined"
          color="success"
          onClick={() => {
            const el = document.createElement('a')
            el.setAttribute(
              'href',
              `data:application/json;chartset=utf-8,${encodeURIComponent(
                JSON.stringify(filtered),
              )}`,
            )
            el.setAttribute('download', 'geojson.json')
            el.style.display = 'none'
            document.body.appendChild(el)
            el.click()
            document.body.removeChild(el)
          }}
        >
          Download
        </Button>
      </Grid2>
    </Grid2>
  )
}
