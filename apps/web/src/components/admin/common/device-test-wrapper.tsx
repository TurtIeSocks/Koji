import { useEffect, type ReactNode } from "react";

const WIDTH_MAP = {
  xs: 0,
  sm: 600,
  md: 900,
  lg: 1200,
  xl: 1536,
} as const;

interface DeviceTestWrapperProps {
  width: keyof typeof WIDTH_MAP;
  children: ReactNode;
}

const parseQuery = (query: string): ((width: number) => boolean) | null => {
  const minMatch = query.match(/\(min-width:\s*(\d+)px\)/);
  const maxMatch = query.match(/\(max-width:\s*(\d+)px\)/);
  if (!minMatch && !maxMatch) {
    return null;
  }
  return (width: number) => {
    if (minMatch && width < parseInt(minMatch[1], 10)) return false;
    if (maxMatch && width > parseInt(maxMatch[1], 10)) return false;
    return true;
  };
};

const buildMatchMedia =
  (width: number) =>
  (query: string): MediaQueryList => {
    const matcher = parseQuery(query);
    const matches = matcher ? matcher(width) : false;
    return {
      matches,
      media: query,
      onchange: null,
      // Deprecated DOM API still part of the MediaQueryList type contract.
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    } as MediaQueryList;
  };

/**
 * Test utility that simulates a fixed device width by overriding
 * `window.matchMedia` and constraining its children to that pixel width.
 *
 * Useful for Storybook and SSR-style tests to verify responsive layouts
 * without resizing the actual viewport.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/device-test-wrapper DeviceTestWrapper documentation}
 *
 * @example
 * <DeviceTestWrapper width="sm">
 *   <MyResponsiveComponent />
 * </DeviceTestWrapper>
 */
function DeviceTestWrapper({ width, children }: DeviceTestWrapperProps) {
  const pixelWidth = WIDTH_MAP[width];

  useEffect(() => {
    const original = window.matchMedia;
    window.matchMedia = buildMatchMedia(pixelWidth) as typeof window.matchMedia;
    return () => {
      window.matchMedia = original;
    };
  }, [pixelWidth]);

  return <div style={{ width: pixelWidth }}>{children}</div>;
}

export { DeviceTestWrapper, type DeviceTestWrapperProps };
