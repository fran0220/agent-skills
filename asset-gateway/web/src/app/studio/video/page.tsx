"use client";

import { motion } from "framer-motion";
import { Clapperboard, ImagePlus, LoaderCircle, Play } from "lucide-react";
import { GlassCard } from "@/components/glass-card";

export default function VideoStudioPage() {
  return (
    <div className="min-h-screen bg-zinc-950 px-4 py-5 text-white lg:px-6">
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-5">
          <GlassCard className="relative overflow-hidden p-4">
            <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(167,139,250,0.16),_transparent_34%),linear-gradient(180deg,rgba(9,9,11,0.35),rgba(9,9,11,0.85))]" />
            <div className="relative rounded-[30px] border border-white/8 bg-zinc-950/60 p-4">
              <div className="mb-4 flex items-center justify-between">
                <div>
                  <p className="text-xs uppercase tracking-[0.3em] text-zinc-500">Video Studio</p>
                  <h1 className="mt-3 text-4xl leading-none">Move the frame</h1>
                </div>
                <Clapperboard className="h-5 w-5 text-violet-300" />
              </div>

              <div className="relative grid min-h-[460px] place-items-center overflow-hidden rounded-[28px] border border-white/10 bg-[linear-gradient(145deg,#1e1b4b,#09090b)]">
                <motion.div
                  animate={{ scale: [1, 1.08, 1], opacity: [0.9, 1, 0.9] }}
                  transition={{ duration: 2.4, repeat: Number.POSITIVE_INFINITY }}
                  className="absolute right-8 top-8 flex h-28 w-28 items-center justify-center rounded-full border border-violet-300/20 bg-violet-300/8"
                >
                  <svg viewBox="0 0 120 120" className="h-full w-full -rotate-90">
                    <circle cx="60" cy="60" r="48" stroke="rgba(255,255,255,0.08)" strokeWidth="8" fill="none" />
                    <circle
                      cx="60"
                      cy="60"
                      r="48"
                      stroke="#c4b5fd"
                      strokeWidth="8"
                      fill="none"
                      strokeLinecap="round"
                      strokeDasharray="301.59"
                      strokeDashoffset="118"
                    />
                  </svg>
                  <LoaderCircle className="absolute h-6 w-6 animate-spin text-violet-200" />
                </motion.div>

                <div className="grid place-items-center gap-4 text-center">
                  <div className="flex h-20 w-20 items-center justify-center rounded-full border border-white/12 bg-black/35 backdrop-blur-md">
                    <Play className="h-7 w-7 text-white" fill="currentColor" />
                  </div>
                  <div>
                    <p className="text-xs uppercase tracking-[0.3em] text-violet-200/70">preview</p>
                    <p className="mt-3 text-2xl text-white">Motion test loop · 46%</p>
                  </div>
                </div>
              </div>
            </div>
          </GlassCard>

          <GlassCard className="grid gap-5 p-5 lg:grid-cols-[280px_minmax(0,1fr)]">
            <label className="grid min-h-56 place-items-center rounded-[26px] border border-dashed border-white/16 bg-white/[0.03] p-6 text-center">
              <input type="file" className="hidden" />
              <div>
                <ImagePlus className="mx-auto h-8 w-8 text-violet-300" />
                <p className="mt-4 text-sm text-white">Reference still</p>
                <p className="mt-2 text-xs uppercase tracking-[0.24em] text-zinc-500">drop frame for I2V</p>
              </div>
            </label>

            <div className="space-y-4">
              <label className="block space-y-2">
                <span className="text-xs uppercase tracking-[0.24em] text-zinc-500">Prompt</span>
                <textarea
                  defaultValue="Orbit around a chrome monolith, camera glides through fog, subtle lens bloom"
                  className="min-h-40 w-full rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none"
                />
              </label>
              <button className="rounded-[24px] border border-white/12 bg-white px-5 py-4 text-sm font-semibold text-zinc-950">
                Generate clip
              </button>
            </div>
          </GlassCard>
        </div>

        <GlassCard className="p-5">
          <div>
            <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Parameters</p>
            <p className="mt-2 text-xl text-white">Motion dial</p>
          </div>

          <div className="mt-6 space-y-5">
            {[
              ["Duration", "6s", 60],
              ["Motion strength", "0.78", 78],
              ["Stylization", "0.42", 42],
            ].map(([label, value, progress]) => (
              <div key={label} className="space-y-2">
                <div className="flex items-center justify-between text-sm text-zinc-400">
                  <span>{label}</span>
                  <span className="text-white">{value}</span>
                </div>
                <input type="range" defaultValue={progress} className="w-full accent-violet-300" />
              </div>
            ))}
          </div>
        </GlassCard>
      </div>
    </div>
  );
}
