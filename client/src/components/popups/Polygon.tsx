import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { Button } from '@mui/material'
import inside from '@turf/boolean-point-in-polygon'
import useDeepCompareEffect from 'use-deep-compare-effect'
import { Popup } from 'react-leaflet'

import { useStatic } from '@hooks/useStatic'
import ExportPolygon from '@components/interface/dialogs/Polygon'

export default function PolygonPopup() {
  const popupLocation = useStatic((s) => s.popupLocation)
  const activeLayer = useStatic((s) => s.activeLayer)

  const pokestops = useStatic((s) => s.pokestops)
  const gyms = useStatic((s) => s.gyms)
  const spawnpoints = useStatic((s) => s.spawnpoints)

  const [open, setOpen] = React.useState('')
  const [activePokestops, setActivePokestops] = React.useState<number | null>(
    null,
  )
  const [activeGyms, setActiveGyms] = React.useState<number | null>(null)
  const [activeSpawnpoints, setActiveSpawnpoints] = React.useState<
    number | null
  >(null)

  const feature = activeLayer ? activeLayer.toGeoJSON() : null

  useDeepCompareEffect(() => {
    if (feature) {
      setActivePokestops(
        feature
          ? pokestops.filter((x) =>
              inside([x.position[1], x.position[0]], feature),
            ).length
          : 0,
      )
    }
  }, [feature || {}, pokestops.length])

  useDeepCompareEffect(() => {
    if (feature) {
      setActiveGyms(
        feature
          ? gyms.filter((x) => inside([x.position[1], x.position[0]], feature))
              .length
          : 0,
      )
    }
  }, [feature || {}, gyms.length])

  useDeepCompareEffect(() => {
    if (feature) {
      setActiveSpawnpoints(
        feature
          ? spawnpoints.filter((x) =>
              inside([x.position[1], x.position[0]], feature),
            ).length
          : 0,
      )
    }
  }, [feature || {}, spawnpoints.length])

  return feature ? (
    <Popup position={popupLocation}>
      {typeof activePokestops === 'number' &&
      typeof activeGyms === 'number' &&
      typeof activeSpawnpoints === 'number' ? (
        <>
          <Grid2 container spacing={2} minWidth={150}>
            <Grid2 xs={12}>Pokestops: {activePokestops.toLocaleString()}</Grid2>
            <Grid2 xs={12}>Gyms: {activeGyms.toLocaleString()}</Grid2>
            <Grid2 xs={12}>
              Spawnpoints: {activeSpawnpoints.toLocaleString()}
            </Grid2>
            <Grid2>
              <Button onClick={() => setOpen('export')}>Export Polygon</Button>
            </Grid2>
          </Grid2>
          {open && (
            <ExportPolygon
              mode="export"
              open={open}
              setOpen={setOpen}
              feature={feature}
            />
          )}
        </>
      ) : null}
    </Popup>
  ) : null
}
