/* eslint-disable @typescript-eslint/ban-ts-comment */
/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable no-param-reassign */

// TODO: WRITE BETTER TS

import { useEffect, useState } from 'react'
import geohash from 'ngeohash'
import seed from 'seedrandom'
import { shallow } from 'zustand/shallow'

import 'leaflet-pixi-overlay'
import * as L from 'leaflet'
import * as PIXI from 'pixi.js'
import { useMap } from 'react-leaflet'

import { PixiMarker } from '@assets/types'

import { ICON_SVG } from '../assets/constants'
import { usePersist } from './usePersist'

const colorMap: Map<string, string> = new Map()

PIXI.settings.FAIL_IF_MAJOR_PERFORMANCE_CAVEAT = false
PIXI.utils.skipHello()

const PIXILoader = PIXI.Loader.shared

function getHashSvg(hash: string) {
  let color = colorMap.get(hash)
  if (!color) {
    const rng = seed(hash)
    color = `#${rng().toString(16).slice(2, 8)}`
    colorMap.set(hash, color)
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" id="${hash}" width="15" height="15" viewBox="-2 -2 24 24">
  <circle cx="10" cy="10" r="10" fill="${color}" fill-opacity="0.8" stroke="black" stroke-width="1" />
  <circle cx="10" cy="10" r="1" fill="black" />
</svg>`
}

function getEncodedIcon(svg: string) {
  const decoded = unescape(encodeURIComponent(svg))
  const base64 = btoa(decoded)
  return `data:image/svg+xml;base64,${base64}`
}

export default function usePixi(markers: PixiMarker[]) {
  const [pixiOverlay, setPixiOverlay] = useState(null)
  const [loaded, setLoaded] = useState(false)
  const map = useMap()
  const [colorByGeohash, geohashPrecision, scaleMarkers] = usePersist(
    (s) => [s.colorByGeohash, s.geohashPrecision, s.scaleMarkers],
    shallow,
  )
  const zoom = scaleMarkers ? Math.min(18, map.getZoom()) : 16
  if (zoom === undefined) {
    // this if statement is to avoid getContainer error
    // map must have zoom prop
    return null
  }

  // load sprites
  useEffect(() => {
    // cancel loading if already loading as it may cause an error
    if (PIXILoader.loading) {
      PIXILoader.reset()
    }
    let loadingAny = false
    for (let i = 0; i < markers.length; i += 1) {
      const resolvedMarkerId = colorByGeohash
        ? geohash.encode(...markers[i].p, geohashPrecision)
        : markers[i].i[0]
      // skip if no ID or already cached
      if (resolvedMarkerId && !PIXILoader.resources[resolvedMarkerId]) {
        loadingAny = true
        PIXILoader.add(
          resolvedMarkerId,
          getEncodedIcon(
            colorByGeohash
              ? getHashSvg(resolvedMarkerId)
              : ICON_SVG[markers[i].i[0]] || '',
          ),
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
    // @ts-ignore
    const overlay = L.pixiOverlay((utils) => {
      const scale = utils.getScale(zoom)
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
      // @ts-ignore
      const container = utils.getContainer()
      // @ts-ignore
      const renderer = utils.getRenderer()
      // @ts-ignore
      const project = utils.latLngToLayerPoint
      // @ts-ignore
      const scale = utils.getScale(zoom)
      for (let j = 0; j < markers.length; j += 1) {
        const { i, p } = markers[j]
        const resolvedIconId = colorByGeohash
          ? geohash.encode(...p, geohashPrecision)
          : i[0]
        if (PIXILoader.resources[resolvedIconId]) {
          const { texture } = PIXILoader.resources[resolvedIconId]
          if (texture) {
            const markerSprite = PIXI.Sprite.from(texture)
            markerSprite.anchor.set(0.5, 0.5)
            const markerCoords = project(p)
            markerSprite.x = markerCoords.x
            markerSprite.y = markerCoords.y
            markerSprite.scale.set(1 / scale)
            container.addChild(markerSprite)
          }
        }
      }
      renderer.render(container)
    }
    return () => {
      if (pixiOverlay) {
        // @ts-ignore
        pixiOverlay.utils.getContainer().removeChildren()
      }
    }
  }, [pixiOverlay, markers, loaded])

  return null
}
