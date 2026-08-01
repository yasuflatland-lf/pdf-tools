import { useLayoutEffect, useRef } from "react";
import { rotationDegrees } from "../lib/rotation";

interface RotatedThumbnailProps {
  alt: string;
  rotation: number;
  src: string;
}

/**
 * Rotates cached thumbnail bytes without changing the preview's geometry. Odd
 * quarter turns exchange the full-size image box's axes, so a scale derived
 * from the fixed preview keeps the transformed box inside it.
 */
export function RotatedThumbnail({ alt, rotation, src }: RotatedThumbnailProps) {
  // `rotation` arrives from PageSlotDto, which Rust writes with
  // Rotation::quarter_turns(): already reduced to 0..4.
  const previewRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const preview = previewRef.current;
    if (!preview) {
      return;
    }

    const fit = () => {
      const { clientHeight, clientWidth } = preview;
      const scale =
        rotation % 2 === 1 && clientHeight > 0 && clientWidth > 0
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
  }, [rotation]);

  return (
    <div ref={previewRef} className="grid h-full w-full place-items-center overflow-hidden">
      <img
        className="h-full w-full object-contain"
        src={src}
        alt={alt}
        style={{
          transform: `rotate(${rotationDegrees(rotation)}deg) scale(var(--thumbnail-rotation-scale, 1))`,
          transformOrigin: "center",
        }}
      />
    </div>
  );
}
