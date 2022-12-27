import { useEffect } from 'react'
import { useMap } from 'react-leaflet'
import { usePersist } from './usePersist'

export default function useLayers() {
  const showCircles = usePersist((s) => s.showCircles)
  const showLines = usePersist((s) => s.showLines)
  const showPolygons = usePersist((s) => s.showPolygons)

  const map = useMap()

  const setPane = (name: string, show: boolean) => {
    const pane = map.getPane(name)
    if (pane) {
      if (show) {
        pane.hidden = false
      } else {
        pane.hidden = true
      }
    }
  }

  useEffect(() => {
    setPane('circles', showCircles)
  }, [showCircles])

  useEffect(() => {
    setPane('lines', showLines)
  }, [showLines])

  useEffect(() => {
    setPane('polygons', showPolygons)
  }, [showPolygons])
}
