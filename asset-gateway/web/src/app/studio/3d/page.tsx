"use client";

import { motion } from "framer-motion";
import { Box, ImagePlus, Orbit, Rotate3D } from "lucide-react";
import { GlassCard } from "@/components/glass-card";
import { pipelineSteps } from "@/lib/mock-data";
import { cn } from "@/lib/utils";

const exportFormats = ["GLB", "FBX", "USDZ", "OBJ"];
const generationModes = [
  { label: "Text → 3D", active: true },
  { label: "Image → 3D", active: false },
];

export default function Viewer3DPage() {
  return (
    <div className="min-h-screen bg-zinc-950 px-4 py-5 text-white lg:px-6">
      <div className="grid gap-5 xl:grid-cols-[340px_minmax(0,1fr)]">
        <GlassCard className="p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.3em] text-zinc-500">3D Viewer</p>
              <h1 className="mt-3 text-4xl leading-none">From prompt to rig</h1>
            </div>
            <Box className="h-5 w-5 text-blue-300" />
          </div>

          <div className="mt-6 grid gap-3">
            {generationModes.map(({ label, active }) => (
              <button
                key={label}
                className={cn(
                  "rounded-[22px] border px-4 py-4 text-left transition",
                  active
                    ? "border-blue-400/30 bg-blue-400/12 text-white"
                    : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                )}
              >
                {label}
              </button>
            ))}
          </div>

          <textarea
            defaultValue="Floating shrine core, segmented hard-surface shell, glowing seams, game-ready proportions"
            className="mt-4 min-h-40 w-full rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none"
          />

          <label className="mt-4 grid min-h-40 place-items-center rounded-[24px] border border-dashed border-white/16 bg-white/[0.03] text-center">
            <input type="file" className="hidden" />
            <div>
              <ImagePlus className="mx-auto h-7 w-7 text-blue-300" />
              <p className="mt-3 text-sm text-white">Upload reference</p>
            </div>
          </label>
        </GlassCard>

        <div className="space-y-5">
          <GlassCard className="p-5">
            <div className="relative grid min-h-[520px] place-items-center overflow-hidden rounded-[30px] border border-white/8 bg-[radial-gradient(circle_at_top,_rgba(96,165,250,0.18),transparent_28%),linear-gradient(180deg,#111827,#09090b)]">
              <motion.div
                animate={{ rotate: 360 }}
                transition={{ duration: 16, repeat: Number.POSITIVE_INFINITY, ease: "linear" }}
                className="grid h-44 w-44 place-items-center rounded-[36px] border border-blue-300/20 bg-blue-300/8 shadow-[0_0_80px_rgba(96,165,250,0.18)]"
              >
                <Rotate3D className="h-20 w-20 text-blue-200" />
              </motion.div>

              <div className="absolute left-6 top-6 flex items-center gap-3 rounded-full border border-white/10 bg-black/35 px-4 py-2 text-sm text-white backdrop-blur-md">
                <Orbit className="h-4 w-4 text-blue-300" />
                drag to orbit
              </div>
            </div>
          </GlassCard>

          <GlassCard className="p-5">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Pipeline</p>
                <p className="mt-1 text-sm text-white">Tripo chain</p>
              </div>
              <Box className="h-4 w-4 text-blue-300" />
            </div>

            <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_280px]">
              <div className="flex items-center justify-between gap-2 overflow-x-auto pb-2">
                {pipelineSteps.map((step, index) => (
                  <div key={step} className="flex items-center gap-2">
                    <div className="grid place-items-center gap-2">
                      <div
                        className={cn(
                          "grid h-11 w-11 place-items-center rounded-full border text-sm",
                          index < 3
                            ? "border-blue-400/30 bg-blue-400/12 text-white"
                            : "border-white/8 bg-white/[0.04] text-zinc-500",
                        )}
                      >
                        {index + 1}
                      </div>
                      <span className="text-xs uppercase tracking-[0.2em] text-zinc-500">{step}</span>
                    </div>
                    {index < pipelineSteps.length - 1 ? <div className="h-px w-12 bg-white/10" /> : null}
                  </div>
                ))}
              </div>

              <div className="grid grid-cols-2 gap-2">
                {exportFormats.map((format, index) => (
                  <button
                    key={format}
                    className={cn(
                      "rounded-[22px] border px-4 py-6 text-center transition",
                      index === 0
                        ? "border-blue-400/30 bg-blue-400/12 text-white"
                        : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                    )}
                  >
                    {format}
                  </button>
                ))}
              </div>
            </div>
          </GlassCard>
        </div>
      </div>
    </div>
  );
}
