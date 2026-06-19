import L from "leaflet";
import "leaflet/dist/leaflet.css";

const MarkerIcon = L.divIcon({
  className: "shadcn-leaflet-marker",
  html: `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="32" viewBox="0 0 24 32" fill="hsl(217 91% 60%)" stroke="white" stroke-width="2"><path d="M12 0C5.4 0 0 5.4 0 12c0 8 12 20 12 20s12-12 12-20C24 5.4 18.6 0 12 0z"/><circle cx="12" cy="12" r="4" fill="white" stroke="none"/></svg>`,
  iconSize: [24, 32],
  iconAnchor: [12, 32],
  popupAnchor: [0, -32],
});

const DEFAULT_TILE_URL = "https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png";

const DEFAULT_ATTRIBUTION =
  '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors';

type LatLngTuple = [number, number];
type GeoJsonCoord = [number, number]; // [lng, lat]

const latLngToCoord = (ll: LatLngTuple): GeoJsonCoord => [ll[1], ll[0]];
const coordToLatLng = (c: GeoJsonCoord): LatLngTuple => [c[1], c[0]];

export {
  MarkerIcon,
  DEFAULT_TILE_URL,
  DEFAULT_ATTRIBUTION,
  type LatLngTuple,
  type GeoJsonCoord,
  latLngToCoord,
  coordToLatLng,
};
