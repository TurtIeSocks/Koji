/* eslint-disable no-console */
import * as React from 'react'
import { Button, Dialog, DialogActions, DialogContent } from '@mui/material'
import type { Feature, FeatureCollection } from 'geojson'
import useDeepCompareEffect from 'use-deep-compare-effect'

import { save } from '@services/fetches'
import DialogHeader from './Header'
import { Code } from '../Code'

interface Props {
  open: string
  setOpen: (open: string) => void
  geojson: FeatureCollection
}

export default function Manager({ open, setOpen, geojson }: Props) {
  const [code, setCode] = React.useState<string>(
    JSON.stringify(geojson, null, 2),
  )

  useDeepCompareEffect(() => {
    setCode(JSON.stringify(geojson, null, 2))
  }, [geojson])

  return (
    <Dialog open={open === 'rawManager'} fullScreen onClose={() => setOpen('')}>
      <DialogHeader action={() => setOpen('')}>Manager</DialogHeader>
      <DialogContent sx={{ margin: 0, padding: 0 }}>
        <Code code={code} setCode={setCode} />
      </DialogContent>
      <DialogActions>
        <Button
          onClick={() => {
            const split: FeatureCollection = JSON.parse(code)
            const features: Feature[] = []
            split.features.forEach((feature: Feature) => {
              if (feature.geometry.type === 'MultiPolygon') {
                const { coordinates } = feature.geometry
                coordinates.forEach((polygon, i) => {
                  features.push({
                    ...feature,
                    properties: {
                      ...feature.properties,
                      name:
                        coordinates.length === 1
                          ? feature.properties?.name || ''
                          : `${feature.properties?.name}_${i}`,
                    },
                    geometry: {
                      ...feature.geometry,
                      type: 'Polygon',
                      coordinates: polygon,
                    },
                  })
                })
              } else {
                features.push(feature)
              }
              setCode(JSON.stringify({ ...split, features }, null, 2))
            })
          }}
        >
          Split Multi Polygons
        </Button>
        <Button
          onClick={() =>
            save('/api/v1/geofence/save-koji', code).then((res) =>
              console.log(res),
            )
          }
        >
          Save to Koji
        </Button>
        <Button
          onClick={() =>
            save('/api/v1/geofence/save-scanner', code).then((res) =>
              console.log(res),
            )
          }
        >
          Save to Scanner
        </Button>
        <Button
          onClick={() => {
            setOpen('')
          }}
        >
          Close
        </Button>
      </DialogActions>
    </Dialog>
  )
}
