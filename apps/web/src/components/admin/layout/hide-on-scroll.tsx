import { useEffect, useRef, useState, type ReactNode } from "react";

import { cn } from "@/lib/utils";

interface HideOnScrollProps {
  children: ReactNode;
  className?: string;
  /**
   * Pixel distance from the top of the viewport before hiding kicks in.
   * @default 100
   */
  threshold?: number;
}

/**
 * Wrapper that hides its children when the user scrolls down past a
 * threshold and reveals them again when scrolling back up. Useful for
 * collapsible top bars and floating headers.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/hide-on-scroll HideOnScroll documentation}
 *
 * @example
 * import { HideOnScroll } from "@/components/admin/layout/hide-on-scroll";
 *
 * const AppBar = () => (
 *   <HideOnScroll>
 *     <header className="sticky top-0">...</header>
 *   </HideOnScroll>
 * );
 */
function HideOnScroll({
  children,
  className,
  threshold = 100,
}: HideOnScrollProps) {
  const [hidden, setHidden] = useState(false);
  const lastScrollY = useRef(
    typeof window !== "undefined" ? window.scrollY : 0,
  );

  useEffect(() => {
    const handleScroll = () => {
      const currentScrollY = window.scrollY;
      const scrollingDown = currentScrollY > lastScrollY.current;
      if (scrollingDown && currentScrollY > threshold) {
        setHidden(true);
      } else if (!scrollingDown) {
        setHidden(false);
      }
      lastScrollY.current = currentScrollY;
    };
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, [threshold]);

  return (
    <div
      className={cn(
        "transition-transform duration-300",
        hidden ? "-translate-y-full opacity-0" : "translate-y-0 opacity-100",
        className,
      )}
    >
      {children}
    </div>
  );
}

export { HideOnScroll, type HideOnScrollProps };
