"use client";

import { useEffect, useMemo, useRef } from "react";
import { cn } from "@/lib/utils";

type AudioWaveformProps = {
  audioUrl?: string;
  className?: string;
  height?: number;
  barColor?: string;
  progressColor?: string;
};

export function AudioWaveform({
  audioUrl,
  className,
  height = 88,
  barColor = "rgba(255,255,255,0.28)",
  progressColor = "rgba(255,255,255,0.88)",
}: AudioWaveformProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const bars = useMemo(
    () => [18, 34, 56, 72, 40, 64, 52, 26, 44, 68, 50, 30, 60, 74, 42, 22],
    [],
  );

  useEffect(() => {
    if (!audioUrl || !containerRef.current) {
      return;
    }

    let mounted = true;
    let cleanup: (() => void) | undefined;

    import("wavesurfer.js").then(({ default: WaveSurfer }) => {
      if (!mounted || !containerRef.current) {
        return;
      }

      const wave = WaveSurfer.create({
        container: containerRef.current,
        height,
        barWidth: 3,
        barGap: 2,
        waveColor: barColor,
        progressColor,
        cursorWidth: 0,
        normalize: true,
      });

      wave.load(audioUrl).catch(() => undefined);
      cleanup = () => wave.destroy();
    });

    return () => {
      mounted = false;
      cleanup?.();
    };
  }, [audioUrl, barColor, height, progressColor]);

  if (!audioUrl) {
    return (
      <div className={cn("flex items-end gap-1.5", className)} style={{ height }}>
        {bars.map((bar, index) => (
          <span
            key={`${bar}-${index}`}
            className="w-full rounded-full bg-white/10"
            style={{
              height: `${bar}%`,
              animation: `wavePulse ${1.2 + index * 0.04}s ease-in-out ${index * 0.06}s infinite alternate`,
            }}
          />
        ))}
      </div>
    );
  }

  return <div ref={containerRef} className={cn("overflow-hidden rounded-2xl", className)} />;
}
