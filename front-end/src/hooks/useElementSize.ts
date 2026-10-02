import { useLayoutEffect, useState } from "react";

/// Tracks an element's content-box size with a ResizeObserver.
export function useElementSize<T extends HTMLElement>(): [
  (node: T | null) => void,
  { width: number; height: number },
] {
  const [node, setNode] = useState<T | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useLayoutEffect(() => {
    if (!node) return;
    const measure = () => {
      const rect = node.getBoundingClientRect();
      setSize((prev) =>
        prev.width === rect.width && prev.height === rect.height
          ? prev
          : { width: rect.width, height: rect.height },
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, [node]);

  return [setNode, size];
}
