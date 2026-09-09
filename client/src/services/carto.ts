/** Append CARTO basemap API key to cartocdn tile URLs when missing. */
export function withCartoKey(url: string, key?: string | null): string {
  if (!url || !key) return url
  if (!url.includes('basemaps.cartocdn.com')) return url
  if (/[?&]key=/.test(url)) return url
  return url.includes('?') ? `${url}&key=${key}` : `${url}?key=${key}`
}