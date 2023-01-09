import Button from '@mui/material/Button'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import * as React from 'react'

import { useShapes } from '@hooks/useShapes'
import type { FeatureCollection } from 'geojson'
import { save } from '@services/fetches'

interface Props {
  code: string
  filtered: FeatureCollection
  reset: () => void
}

export default function FinishStep({ code, filtered, reset }: Props) {
  return (
    <Grid2
      container
      minHeight="50vh"
      width="100%"
      direction="column"
      justifyContent="space-around"
    >
      <Grid2>
        <Button
          variant="outlined"
          color="success"
          onClick={() =>
            save('/api/v1/geofence/save-koji', code).then((res) =>
              // eslint-disable-next-line no-console
              console.log(res),
            )
          }
        >
          Save to Kōji Database
        </Button>
      </Grid2>
      <Grid2>
        <Button
          variant="outlined"
          color="success"
          onClick={() => {
            useShapes.getState().setters.add(filtered.features)
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
                code,
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
