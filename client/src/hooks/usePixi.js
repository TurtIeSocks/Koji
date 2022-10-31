/* eslint-disable no-param-reassign */

// TODO: CONVERT TO TS

import { useEffect, useState } from 'react'

import * as L from 'leaflet'
import * as PIXI from 'pixi.js'
import 'leaflet-pixi-overlay'
import { useMap } from 'react-leaflet'

import { ICON_SVG } from '../assets/constants'

function openPopup(map, data, extraOptions = {}, isPopup = false) {
  const popup = L.popup({ offset: data.offset, ...extraOptions })
    .setLatLng(data.position)
    .setContent(data.content)
    .addTo(map)
  if (isPopup) {
    popup.on('remove', () => {
      if (data.onClick) {
        data.onClick()
      }
    })
  }
  return popup
}

function openTooltip(map, data) {
  const tooltip = L.tooltip({ offset: data.offset, ...data.tooltipOptions })
    .setLatLng(data.position)
    .setContent(data.content)
    .addTo(map)
  return tooltip
}

function getEncodedIcon(svg) {
  const decoded = unescape(encodeURIComponent(svg))
  const base64 = btoa(decoded)
  return `data:image/svg+xml;base64,${base64}`
}

PIXI.settings.FAIL_IF_MAJOR_PERFORMANCE_CAVEAT = false
PIXI.utils.skipHello()
const PIXILoader = PIXI.Loader.shared

export default function usePixi(markers) {
  const [openedPopupData, setOpenedPopupData] = useState(null)
  const [openedTooltipData, setOpenedTooltipData] = useState(null)
  const [openedPopup, setOpenedPopup] = useState(null)
  const [openedTooltip, setOpenedTooltip] = useState(null)
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
    const overlay = L.pixiOverlay((utils) => {
      // redraw markers
      const scale = utils.getScale(16)
      utils.getContainer().children.forEach((child) => {
        child.currentScale = child.scale.x
        child.targetScale = 1 / scale
      })
      utils.getRenderer().render(utils.getContainer())
    }, pixiContainer)
    overlay.addTo(map)
    setPixiOverlay(overlay)
    setOpenedPopupData(null)
    setOpenedTooltipData(null)
    return () => {
      pixiContainer.removeChildren()
    }
  }, [map])

  // draw markers first time in new container
  useEffect(() => {
    if (pixiOverlay && markers && loaded) {
      const { utils } = pixiOverlay
      const container = utils.getContainer()
      const renderer = utils.getRenderer()
      const project = utils.latLngToLayerPoint
      const scale = utils.getScale(16)
      markers.forEach((marker) => {
        const {
          i,
          onClick,
          p,
          popup,
          tooltip,
          tooltipOptions,
          popupOpen,
          markerSpriteAnchor,
          angle,
        } = marker
        const resolvedIconId = i[0]
        if (
          !PIXILoader.resources[`marker_${resolvedIconId}`] ||
          !PIXILoader.resources[`marker_${resolvedIconId}`].texture
        ) {
          return
        }
        const markerTexture =
          PIXILoader.resources[`marker_${resolvedIconId}`].texture
        // const markerTexture = new PIXI.Texture.fromImage(url);
        if (markerTexture) {
          markerTexture.anchor = {
            x: 0.5,
            y: 1,
          }
          const markerSprite = PIXI.Sprite.from(markerTexture)
          if (markerSpriteAnchor) {
            markerSprite.anchor.set(
              markerSpriteAnchor[0],
              markerSpriteAnchor[1],
            )
          } else {
            markerSprite.anchor.set(0.5, 0.5)
          }
          const markerCoords = project(p)
          markerSprite.x = markerCoords.x
          markerSprite.y = markerCoords.y
          if (angle) {
            markerSprite.angle = angle
          }
          markerSprite.scale.set(1 / scale)
          if (popupOpen) {
            setOpenedPopupData({
              id: i,
              offset: [0, -35],
              position: p,
              content: popup || '',
              onClick,
            })
          }
          if (popup || onClick || tooltip) {
            markerSprite.interactive = true
          }
          if (popup || onClick) {
            // Prevent accidental launch of onClick event when dragging the map.
            // Detect very small moves as clicks.
            markerSprite.on('mousedown', () => {
              let moveCount = 0
              markerSprite.on('mousemove', () => {
                moveCount += 1
              })
              markerSprite.on('mouseup', () => {
                if (moveCount < 2 && onClick) {
                  onClick(i)
                }
              })
            })
            // Prevent the same thing on touch devices.
            markerSprite.on('touchstart', () => {
              let moveCount = 0
              markerSprite.on('touchmove', () => {
                moveCount += 1
              })
              markerSprite.on('touchend', () => {
                if (moveCount < 10 && onClick) {
                  onClick(i)
                }
              })
            })
            markerSprite.defaultCursor = 'pointer'
            markerSprite.buttonMode = true
          }
          if (tooltip) {
            markerSprite.on('mouseover', () => {
              setOpenedTooltipData({
                id: i,
                offset: [0, -35],
                position: p,
                content: tooltip,
                tooltipOptions: tooltipOptions || {},
              })
            })
            markerSprite.on('mouseout', () => {
              setOpenedTooltipData(null)
            })
          }
          container.addChild(markerSprite)
        }
      })
      renderer.render(container)
    }
    return () => {
      if (pixiOverlay) {
        pixiOverlay.utils.getContainer().removeChildren()
      }
    }
  }, [pixiOverlay, markers, loaded])
  // handle tooltip
  useEffect(() => {
    if (openedTooltip) {
      map.removeLayer(openedTooltip)
    }
    if (
      openedTooltipData &&
      (!openedPopup ||
        !openedPopupData ||
        openedPopupData.id !== openedTooltipData.id)
    ) {
      setOpenedTooltip(openTooltip(map, openedTooltipData))
    }
    // we don't want to reload when openedTooltip changes as we'd get a loop
  }, [openedTooltipData, openedPopupData, map])
  // handle popup
  useEffect(() => {
    // close only if different popup
    if (openedPopup) {
      map.removeLayer(openedPopup)
    }
    // open only if new popup
    if (openedPopupData) {
      setOpenedPopup(
        openPopup(
          map,
          openedPopupData,
          {
            autoClose: false,
          },
          true,
        ),
      )
    }
    // we don't want to reload when whenedPopup changes as we'd get a loop
  }, [openedPopupData, map])
  return null
}
