import { useEffect } from 'react'
import { useMap } from 'react-leaflet'
import { useStore } from './useStore'

export default function useLayers() {
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const showPolygons = useStore((s) => s.showPolygons)

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
