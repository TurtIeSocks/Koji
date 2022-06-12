/* eslint-disable no-param-reassign */
/* eslint-disable no-console */
import { useEffect, useState } from 'react'
import L, { Map, Path, Popup } from 'leaflet'
import * as PIXI from 'pixi.js'
import 'leaflet-pixi-overlay'
import { useMap } from 'react-leaflet'

export interface MarkerPropsPixiOverlay {
  id: string | number
  position: [number, number]
  iconColor?: string
  popup?: string
  popupOpen?: boolean
  onClick?: (id?: string | number) => void
  tooltip?: string
  tooltipOptions?: Path
  customIcon?: string
  iconId?: string
  markerSpriteAnchor?: [number, number]
  angle?: number
}

export interface PopupOptions {
  id: string | number
  offset: [number, number]
  position: [number, number]
  content: string
  tooltipOptions?: Path
  onClick?: (id?: string | number) => void
}

function openPopup(
  map: Map,
  data: PopupOptions,
  extraOptions = {},
  isPopup = false,
) {
  const popup = L.popup({ offset: data.offset, ...extraOptions })
    .setLatLng(data.position)
    .setContent(data.content)
    .addTo(map)
  // TODO don't call onClick if opened a new one
  if (isPopup) {
    popup.on('remove', () => {
      if (data.onClick) {
        data.onClick()
      }
    })
  }
  return popup
}

function openTooltip(map: Map, data: PopupOptions) {
  const tooltip = L.tooltip({ offset: data.offset, ...data.tooltipOptions })
    .setLatLng(data.position)
    .setContent(data.content)
    .addTo(map)
  return tooltip
}

function getEncodedIcon(svg: string) {
  const decoded = unescape(encodeURIComponent(svg))
  const base64 = btoa(decoded)
  return `data:image/svg+xml;base64,${base64}`
}

function getDefaultIcon(color: string) {
  const svgIcon = `<svg style="-webkit-filter: drop-shadow( 1px 1px 1px rgba(0, 0, 0, .4));filter: drop-shadow( 1px 1px 1px rgba(0, 0, 0, .4));" xmlns="http://www.w3.org/2000/svg" fill="${color}" width="36" height="36" viewBox="0 0 24 24"><path d="M12 0c-4.198 0-8 3.403-8 7.602 0 6.243 6.377 6.903 8 16.398 1.623-9.495 8-10.155 8-16.398 0-4.199-3.801-7.602-8-7.602zm0 11c-1.657 0-3-1.343-3-3s1.342-3 3-3 3 1.343 3 3-1.343 3-3 3z"/></svg>`
  return getEncodedIcon(svgIcon)
}

PIXI.settings.FAIL_IF_MAJOR_PERFORMANCE_CAVEAT = false
PIXI.utils.skipHello()
const PIXILoader = PIXI.Loader.shared

export default function usePixi({
  markers,
}: {
  markers: MarkerPropsPixiOverlay[]
}) {
  const [openedPopupData, setOpenedPopupData] = useState<PopupOptions | null>(
    null,
  )
  const [openedTooltipData, setOpenedTooltipData] = useState(null)
  const [openedPopup, setOpenedPopup] = useState(null)
  const [openedTooltip, setOpenedTooltip] = useState<L.Tooltip | null>(null)
  const [pixiOverlay, setPixiOverlay] = useState(null)
  const [loaded, setLoaded] = useState(false)
  const map = useMap()
  if (map.getZoom() === undefined) {
    // this if statement is to avoid getContainer error
    // map must have zoom prop
    console.error(
      'no zoom found, add zoom prop to map to avoid getContainer error',
    )
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
      const resolvedMarkerId = markers[i].iconId || markers[i].iconColor
      // skip if no ID or already cached
      if (
        !PIXILoader.resources[`marker_${resolvedMarkerId}`] &&
        resolvedMarkerId
      ) {
        loadingAny = true
        PIXILoader.add(
          `marker_${resolvedMarkerId}`,
          markers[i].customIcon
            ? getEncodedIcon(markers[i].customIcon || '')
            : getDefaultIcon(markers[i].iconColor || ''),
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
          id,
          iconColor,
          iconId,
          onClick,
          position,
          popup,
          tooltip,
          tooltipOptions,
          popupOpen,
          markerSpriteAnchor,
          angle,
        } = marker
        const resolvedIconId = iconId || iconColor
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
            markerSprite.anchor.set(0.5, 1)
          }
          const markerCoords = project(position)
          markerSprite.x = markerCoords.x
          markerSprite.y = markerCoords.y
          if (angle) {
            markerSprite.angle = angle
          }
          markerSprite.scale.set(1 / scale)
          if (popupOpen) {
            setOpenedPopupData({
              id,
              offset: [0, -35],
              position,
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
                  onClick(id)
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
                  onClick(id)
                }
              })
            })
            markerSprite.defaultCursor = 'pointer'
            markerSprite.buttonMode = true
          }
          if (tooltip) {
            markerSprite.on('mouseover', () => {
              setOpenedTooltipData({
                id,
                offset: [0, -35],
                position,
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
