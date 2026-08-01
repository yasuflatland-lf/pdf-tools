import { useLayoutEffect, useRef } from "react";

interface RotatedThumbnailProps {
  alt: string;
  rotation: number;
  src: string;
}

function normalizedQuarterTurns(rotation: number): number {
  return ((rotation % 4) + 4) % 4;
}

/**
 * Rotates cached thumbnail bytes without changing the preview's geometry. Odd
 * quarter turns exchange the full-size image box's axes, so a scale derived
 * from the fixed preview keeps the transformed box inside it.
 */
export function RotatedThumbnail({ alt, rotation, src }: RotatedThumbnailProps) {
  const previewRef = useRef<HTMLDivElement>(null);
  const quarterTurns = normalizedQuarterTurns(rotation);

  useLayoutEffect(() => {
    const preview = previewRef.current;
    if (!preview) {
      return;
    }

    const fit = () => {
      const { clientHeight, clientWidth } = preview;
      const scale =
        quarterTurns % 2 === 1 && clientHeight > 0 && clientWidth > 0
          ? Math.min(1, clientWidth / clientHeight, clientHeight / clientWidth)
          : 1;
      preview.style.setProperty("--thumbnail-rotation-scale", String(scale));
    };

    fit();
    if (typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver(fit);
    observer.observe(preview);
    return () => observer.disconnect();
  }, [quarterTurns]);

  return (
    <div ref={previewRef} className="grid h-full w-full place-items-center overflow-hidden">
      <img
        className="h-full w-full object-contain"
        src={src}
        alt={alt}
        style={{
          transform: `rotate(${quarterTurns * 90}deg) scale(var(--thumbnail-rotation-scale, 1))`,
          transformOrigin: "center",
        }}
      />
    </div>
  );
}
