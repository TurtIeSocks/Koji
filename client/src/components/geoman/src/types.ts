/* eslint-disable @typescript-eslint/no-explicit-any */
import * as L from 'leaflet'
import type { FeatureCollection } from 'geojson'

export type Method = 'on' | 'off'

export type Fallback = (input?: any) => void

export type HandlersWithFallback = GeomanHandlers & { fallback: Fallback }

export type ValueOf<T extends keyof GeomanHandlers> = GeomanHandlers[T]

export interface GeomanHandlers {
  // global
  onMapRemove?: L.PM.RemoveEventHandler
  onLayerRemove?: L.PM.RemoveEventHandler
  onMapCut?: L.PM.CutEventHandler
  onLayerCut?: L.PM.CutEventHandler
  onMapRotateEnable?: L.PM.RotateEnableEventHandler
  onLayerRotateEnable?: L.PM.RotateEnableEventHandler
  onMapRotateDisable?: L.PM.RotateDisableEventHandler
  onLayerRotateDisable?: L.PM.RotateDisableEventHandler
  onMapRotateStart?: L.PM.RotateStartEventHandler
  onLayerRotateStart?: L.PM.RotateStartEventHandler
  onMapRotate?: L.PM.RotateEventHandler
  onLayerRotate?: L.PM.RotateEventHandler
  onMapRotateEnd?: L.PM.RotateEndEventHandler
  onLayerRotateEnd?: L.PM.RotateEndEventHandler

  // map
  onGlobalDrawModeToggle?: L.PM.GlobalDrawModeToggledEventHandler
  onDrawStart?: L.PM.DrawStartEventHandler
  onDrawEnd?: L.PM.DrawEndEventHandler
  onCreate?: L.PM.CreateEventHandler
  onGlobalEditModeToggled?: L.PM.GlobalEditModeToggledEventHandler
  onGlobalDragModeToggled?: L.PM.GlobalDragModeToggledEventHandler
  onGlobalRemovalModeToggled?: L.PM.GlobalRemovalModeToggledEventHandler
  onGlobalCutModeToggled?: L.PM.GlobalCutModeToggledEventHandler
  onGlobalRotateModeToggled?: L.PM.GlobalRotateModeToggledEventHandler
  onLangChange?: L.PM.LangChangeEventHandler
  onButtonClick?: L.PM.ButtonClickEventHandler
  onActionClick?: L.PM.ActionClickEventHandler
  onKeyEvent?: L.PM.KeyboardKeyEventHandler

  // layer
  onSnapDrag?: L.PM.SnapEventHandler
  onSnap?: L.PM.SnapEventHandler
  onUnsnap?: L.PM.SnapEventHandler
  onCenterPlaced?: L.PM.CenterPlacedEventHandler
  onEdit?: L.PM.EditEventHandler
  onUpdate?: L.PM.UpdateEventHandler
  onEnable?: L.PM.EnableEventHandler
  onDisable?: L.PM.DisableEventHandler
  onVertexAdded?: L.PM.VertexAddedEventHandler
  onVertexRemoved?: L.PM.VertexRemovedEventHandler
  onVertexClick?: L.PM.VertexClickEventHandler
  onMarkerDragStart?: L.PM.MarkerDragStartEventHandler
  onMarkerDrag?: L.PM.MarkerDragEventHandler
  onMarkerDragEnd?: L.PM.MarkerDragEndEventHandler
  onLayerReset?: L.PM.LayerResetEventHandler
  onIntersect?: L.PM.IntersectEventHandler
  onChange?: L.PM.ChangeEventHandler
  onTextChange?: L.PM.TextChangeEventHandler
  onDragStart?: L.PM.DragStartEventHandler
  onDrag?: L.PM.DragEventHandler
  onDragEnd?: L.PM.DragEndEventHandler
  onDragEnable?: L.PM.DragEnableEventHandler
  onDragDisable?: L.PM.DragDisableEventHandler
}

export interface GeomanProps extends GeomanHandlers {
  map: L.Map
  options?: L.PM.ToolbarOptions
  globalOptions?: L.PM.GlobalOptions
  pathOptions?: L.PathOptions
  geojson?: FeatureCollection
  fallback?: Fallback
  lang?:
    | 'cz'
    | 'da'
    | 'de'
    | 'el'
    | 'en'
    | 'es'
    | 'fa'
    | 'fr'
    | 'hu'
    | 'id'
    | 'it'
    | 'nl'
    | 'no'
    | 'pl'
    | 'pt_br'
    | 'ro'
    | 'ru'
    | 'sv'
    | 'tr'
    | 'ua'
    | 'zh'
    | 'zh_tw'
}
