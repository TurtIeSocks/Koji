import { useStore } from '@hooks/useStore'
import { getLotsOfData } from '@services/fetches'
// import { COLORS } from '@assets/constants'
import { useStatic } from '@hooks/useStatic'
import useDeepCompareEffect from 'use-deep-compare-effect'

export default function useCluster(): void {
  const mode = useStore((s) => s.mode)
  const radius = useStore((s) => s.radius)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const devices = useStore((s) => s.devices)
  const tab = useStore((s) => s.tab)
  const min_points = useStore((s) => s.min_points)
  const fast = useStore((s) => s.fast)
  const autoMode = useStore((s) => s.autoMode)
  const routing_time = useStore((s) => s.routing_time)
  const only_unique = useStore((s) => s.only_unique)
  const save_to_db = useStore((s) => s.save_to_db)
  const last_seen = useStore((s) => s.last_seen)
  const route_chunk_size = useStore((s) => s.route_chunk_size)

  const layerEditing = useStatic((s) => s.layerEditing)
  const geojson = useStatic((s) => s.geojson)
  const forceFetch = useStatic((s) => s.forceFetch)
  const setGeojson = useStatic((s) => s.setGeojson)

  useDeepCompareEffect(() => {
    if (geojson.features.length) {
      if (Object.values(layerEditing).every((v) => !v)) {
        getLotsOfData(
          mode === 'bootstrap'
            ? '/api/v1/calc/bootstrap'
            : `/api/v1/calc/${mode}/${category}`,
          {
            category,
            radius,
            generations,
            devices,
            geojson,
            min_points,
            fast,
            routing_time,
            only_unique,
            save_to_db,
            last_seen,
            route_chunk_size,
          },
        ).then((newCollection) => {
          setGeojson(newCollection)
        })
      }
    }
  }, [
    autoMode
      ? {
          mode,
          radius,
          fast,
          generations,
          min_points,
          category,
          devices,
          geojson,
          tab,
          routing_time,
          only_unique,
          save_to_db,
          last_seen,
        }
      : { forceFetch },
  ])
}
