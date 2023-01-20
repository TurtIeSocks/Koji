import * as React from 'react'
import { FormDataConsumer, SelectInput, TextInput } from 'react-admin'
import { Box } from '@mui/material'
import Map from '@components/Map'
import { GeoJSON, useMap } from 'react-leaflet'
import center from '@turf/center'
import { useStatic } from '@hooks/useStatic'
import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import { getColor, safeParse } from '@services/utils'
import type { FeatureCollection, MultiPoint } from 'geojson'
import * as L from 'leaflet'
import distance from '@turf/distance'
import CodeInput from '../inputs/CodeInput'

export function GeoJsonWrapper({
  fc,
  mode,
}: {
  fc: FeatureCollection
  mode: string
}) {
  const map = useMap()
  return (
    <GeoJSON
      data={fc}
      pointToLayer={(feat, latlng) => {
        L.polyline(
          [
            [latlng.lat, latlng.lng],
            [feat.properties?.next[1], feat.properties?.next[0]],
          ],
          {
            color: getColor(
              distance(feat, feat.properties?.next, {
                units: 'meters',
              }),
            ),
          },
        ).addTo(map)
        return L.circle(latlng, {
          radius:
            {
              ManualQuest: 80,
              CircleRaid: 1100,
              CircleSmartRaid: 1100,
            }[mode] || 70,
        })
      }}
    />
  )
}

export default function RouteForm() {
  const scannerType = useStatic((s) => s.scannerType)

  return (
    <>
      <TextInput source="name" fullWidth required />
      <SelectInput
        source="mode"
        choices={(scannerType === 'rdm' ? RDM_ROUTES : UNOWN_ROUTES).map(
          (mode, i) => ({ id: i, mode }),
        )}
        optionText="mode"
        optionValue="mode"
      />
      <FormDataConsumer>
        {({ formData }) => {
          const parsed: MultiPoint =
            typeof formData.geometry === 'string'
              ? (() => {
                  const safe = safeParse<MultiPoint>(formData.geometry)
                  if (!safe.error) return safe.value
                })()
              : formData.geometry
          if (parsed === undefined) return null
          const point = center(parsed)

          const fc: FeatureCollection = {
            type: 'FeatureCollection',
            features: (parsed.coordinates || []).map((c, i) => {
              return {
                type: 'Feature',
                geometry: {
                  type: 'Point',
                  coordinates: c,
                },
                properties: {
                  next: parsed.coordinates[
                    i === parsed.coordinates.length - 1 ? 0 : i + 1
                  ],
                },
              }
            }),
          }
          return (
            <Map
              forcedLocation={[
                point.geometry.coordinates[1],
                point.geometry.coordinates[0],
              ]}
              forcedZoom={8}
              zoomControl
              style={{ width: '100%', height: '50vh' }}
            >
              <GeoJsonWrapper fc={fc} mode={formData.mode} />
            </Map>
          )
        }}
      </FormDataConsumer>
      <Box pt="1em" />
      <CodeInput source="geometry" label="Route" />
    </>
  )
}
