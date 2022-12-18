import { useEffect } from 'react'
import { useMap } from 'react-leaflet'
import { useStore } from './useStore'

export default function useLayers() {
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const showPolygons = useStore((s) => s.showPolygons)

  const map = useMap()

  useEffect(() => {
    const pane = map.getPane('circles')
    if (pane) {
      if (showCircles) {
        pane.hidden = false
      } else {
        pane.hidden = true
      }
    }
  }, [showCircles])

  useEffect(() => {
    const pane = map.getPane('lines')
    if (pane) {
      if (showLines) {
        pane.hidden = false
      } else {
        pane.hidden = true
      }
    }
  }, [showLines])

  useEffect(() => {
    const pane = map.getPane('polygons')
    if (pane) {
      if (showPolygons) {
        pane.hidden = false
      } else {
        pane.hidden = true
      }
    }
  }, [showPolygons])
}
