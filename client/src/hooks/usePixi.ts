/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable no-param-reassign */

// TODO: WRITE BETTER TS

import { useEffect, useState } from 'react'

import * as L from 'leaflet'
import * as PIXI from 'pixi.js'
import 'leaflet-pixi-overlay'
import { useMap } from 'react-leaflet'

import { PixiMarker } from '@assets/types'

import { ICON_SVG } from '../assets/constants'

function getEncodedIcon(svg: string) {
  const decoded = unescape(encodeURIComponent(svg))
  const base64 = btoa(decoded)
  return `data:image/svg+xml;base64,${base64}`
}

PIXI.settings.FAIL_IF_MAJOR_PERFORMANCE_CAVEAT = false
PIXI.utils.skipHello()
const PIXILoader = PIXI.Loader.shared

export default function usePixi(markers: PixiMarker[]) {
  const [pixiOverlay, setPixiOverlay] = useState(null)
  const [loaded, setLoaded] = useState(false)
  const map = useMap()
  if (map.getZoom() === undefined) {
    // this if statement is to avoid getContainer error
    // map must have zoom prop
    return null
  }

  // load sprites
  useEffect(() => {
    // cancel loading if already loading as it may cause: Error: Cannot add resources while the loader is running.
    if (PIXILoader.loading) {
      PIXILoader.reset()
    }
    let loadingAny = false
    for (let i = 0; i < markers.length; i += 1) {
      const resolvedMarkerId = markers[i].i[0]
      // skip if no ID or already cached
      if (
        !PIXILoader.resources[`marker_${resolvedMarkerId}`] &&
        resolvedMarkerId
      ) {
        loadingAny = true
        PIXILoader.add(
          `marker_${resolvedMarkerId}`,
          getEncodedIcon(ICON_SVG[markers[i].i[0]] || ''),
        )
      }
    }
    if (loaded && loadingAny) {
      setLoaded(false)
    }
    if (loadingAny) {
      PIXILoader.load(() => setLoaded(true))
    } else {
      setLoaded(true)
    }
  }, [markers])

  // load pixi when map changes
  useEffect(() => {
    const pixiContainer = new PIXI.Container()
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    const overlay = L.pixiOverlay((utils) => {
      // redraw markers
      const scale = utils.getScale(16)
      utils.getContainer().children.forEach((child: any) => {
        child.currentScale = child.scale.x
        child.targetScale = 1 / scale
      })
      utils.getRenderer().render(utils.getContainer())
    }, pixiContainer)
    overlay.addTo(map)
    setPixiOverlay(overlay)
    return () => {
      pixiContainer.removeChildren()
    }
  }, [map])

  // draw markers first time in new container
  useEffect(() => {
    if (pixiOverlay && markers && loaded) {
      const { utils } = pixiOverlay
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      const container = utils.getContainer()
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      const renderer = utils.getRenderer()
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      const project = utils.latLngToLayerPoint
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      const scale = utils.getScale(16)
      markers.forEach((marker) => {
        const { i, p } = marker
        const resolvedIconId = i[0]
        if (
          !PIXILoader.resources[`marker_${resolvedIconId}`] ||
          !PIXILoader.resources[`marker_${resolvedIconId}`].texture
        ) {
          return
        }
        const markerTexture =
          PIXILoader.resources[`marker_${resolvedIconId}`].texture
        if (markerTexture) {
          const markerSprite = PIXI.Sprite.from(markerTexture)
          markerSprite.anchor.set(0.5, 0.5)
          const markerCoords = project(p)
          markerSprite.x = markerCoords.x
          markerSprite.y = markerCoords.y
          markerSprite.scale.set(1 / scale)
          container.addChild(markerSprite)
        }
      })
      renderer.render(container)
    }
    return () => {
      if (pixiOverlay) {
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        pixiOverlay.utils.getContainer().removeChildren()
      }
    }
  }, [pixiOverlay, markers, loaded])
  return null
}
