import * as React from 'react'

import inside from '@turf/boolean-point-in-polygon'

import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { useStatic } from '@hooks/useStatic'
import { Button } from '@mui/material'
import { Popup } from 'react-leaflet'
import ExportPolygon from '@components/interface/dialogs/ExportPolygon'
import useDeepCompareEffect from 'use-deep-compare-effect'

export function PolygonPopup() {
  const activeLayer = useStatic((s) => s.activeLayer)
  const popupLocation = useStatic((s) => s.popupLocation)

  const pokestops = useStatic((s) => s.pokestops)
  const gyms = useStatic((s) => s.gyms)
  const spawnpoints = useStatic((s) => s.spawnpoints)

  const [open, setOpen] = React.useState(false)
  const [activePokestops, setActivePokestops] = React.useState(0)
  const [activeGyms, setActiveGyms] = React.useState(0)
  const [activeSpawnpoints, setActiveSpawnpoints] = React.useState(0)

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
    <Popup autoClose position={popupLocation}>
      <Grid2 container spacing={2}>
        <Grid2 xs={12}>Pokestops: {activePokestops.toLocaleString()}</Grid2>
        <Grid2 xs={12}>Gyms: {activeGyms.toLocaleString()}</Grid2>
        <Grid2 xs={12}>Spawnpoints: {activeSpawnpoints.toLocaleString()}</Grid2>
        <Grid2>
          <Button onClick={() => setOpen(true)}>Export Polygon</Button>
        </Grid2>
      </Grid2>
      <ExportPolygon open={open} setOpen={setOpen} feature={feature} />
    </Popup>
  ) : null
}
