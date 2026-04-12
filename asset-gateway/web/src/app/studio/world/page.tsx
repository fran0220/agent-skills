"use client";

import { motion } from "framer-motion";
import { Compass, Move3D, Sparkles } from "lucide-react";

export default function WorldExplorerPage() {
  return (
    <div className="starfield relative min-h-screen overflow-hidden bg-zinc-950 text-white">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(45,212,191,0.18),_transparent_28%),radial-gradient(circle_at_bottom,_rgba(34,211,238,0.08),_transparent_35%)]" />

      <motion.div
        animate={{ scale: [1, 1.04, 1], opacity: [0.55, 0.75, 0.55] }}
        transition={{ duration: 6, repeat: Number.POSITIVE_INFINITY }}
        className="absolute left-1/2 top-1/2 h-[520px] w-[520px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-teal-400/10 blur-3xl"
      />

      <div className="relative z-10 flex min-h-screen items-end justify-between gap-6 px-4 py-6 lg:px-8">
        <div className="rounded-[30px] border border-white/10 bg-black/32 p-6 backdrop-blur-2xl lg:w-[420px]">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">World Explorer</p>
              <h1 className="mt-3 text-5xl leading-none">Drop into a splat world.</h1>
            </div>
            <Sparkles className="h-5 w-5 text-teal-300" />
          </div>

          <textarea
            defaultValue="Monolithic canyon city with suspended pathways, reflective pools, thin teal aurora"
            className="mt-6 min-h-36 w-full rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none"
          />
          <button className="mt-4 w-full rounded-[24px] border border-white/12 bg-white px-5 py-4 text-sm font-semibold text-zinc-950">
            Generate world
          </button>
        </div>

        <div className="hidden rounded-[30px] border border-white/10 bg-black/28 p-5 backdrop-blur-2xl lg:block">
          <div className="flex items-center gap-3 text-sm text-white">
            {["W", "A", "S", "D"].map((key) => (
              <div key={key} className="grid h-11 w-11 place-items-center rounded-2xl border border-white/10 bg-white/[0.04]">
                {key}
              </div>
            ))}
            <div className="ml-3 flex items-center gap-2 text-zinc-400">
              <Move3D className="h-4 w-4 text-teal-300" />
              <Compass className="h-4 w-4 text-teal-300" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
