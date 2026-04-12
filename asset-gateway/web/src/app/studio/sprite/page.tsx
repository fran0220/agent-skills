"use client";

import { motion } from "framer-motion";
import { Pause, Play, Sparkles } from "lucide-react";
import Image from "next/image";
import { GlassCard } from "@/components/glass-card";
import { spriteFrames } from "@/lib/mock-data";
import { cn } from "@/lib/utils";

const animationTypes = ["walk", "run", "idle", "attack"];

export default function SpriteEditorPage() {
  return (
    <div className="min-h-screen bg-zinc-950 px-4 py-5 text-white lg:px-6">
      <div className="grid gap-5 xl:grid-cols-[340px_minmax(0,1fr)]">
        <GlassCard className="p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.3em] text-zinc-500">Sprite Editor</p>
              <h1 className="mt-3 text-4xl leading-none">Loop the character</h1>
            </div>
            <Sparkles className="h-5 w-5 text-orange-300" />
          </div>

          <textarea
            defaultValue="Courier mech with orange jacket accents, readable silhouette, clean side profile"
            className="mt-6 min-h-40 w-full rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none"
          />

          <div className="mt-5 grid gap-2">
            {animationTypes.map((type, index) => (
              <button
                key={type}
                className={cn(
                  "rounded-[22px] border px-4 py-4 text-left capitalize transition",
                  index === 0
                    ? "border-orange-400/30 bg-orange-400/12 text-white"
                    : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                )}
              >
                {type}
              </button>
            ))}
          </div>
        </GlassCard>

        <div className="space-y-5">
          <GlassCard className="p-5">
            <div className="grid min-h-[520px] place-items-center rounded-[30px] border border-white/8 bg-[radial-gradient(circle_at_top,_rgba(251,146,60,0.18),transparent_30%),linear-gradient(180deg,#431407,#09090b)] p-8">
              <div className="grid w-full max-w-3xl gap-4 rounded-[30px] border border-white/10 bg-black/25 p-5 backdrop-blur-md">
                <div className="grid grid-cols-4 gap-3">
                  {spriteFrames.slice(0, 4).map((frame, index) => (
                    <motion.div
                      key={frame.id}
                      animate={{ y: [0, -4, 0] }}
                      transition={{ duration: 0.8, delay: index * 0.08, repeat: Number.POSITIVE_INFINITY }}
                      className="relative overflow-hidden rounded-[24px] border border-white/10 bg-white/[0.04]"
                    >
                      <Image
                        src={frame.preview}
                        alt={frame.id}
                        width={640}
                        height={480}
                        unoptimized
                        className="h-36 w-full object-cover"
                      />
                    </motion.div>
                  ))}
                </div>

                <div className="flex flex-wrap items-center gap-3 rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-3">
                  <button className="flex h-11 w-11 items-center justify-center rounded-full border border-white/10 bg-white/10 text-white">
                    <Play className="h-4 w-4" fill="currentColor" />
                  </button>
                  <button className="flex h-11 w-11 items-center justify-center rounded-full border border-white/10 bg-white/6 text-white">
                    <Pause className="h-4 w-4" />
                  </button>
                  <div className="min-w-48 flex-1">
                    <div className="mb-2 flex items-center justify-between text-sm text-zinc-400">
                      <span>Speed</span>
                      <span className="text-white">1.2x</span>
                    </div>
                    <input type="range" defaultValue={60} className="w-full accent-orange-300" />
                  </div>
                  <div className="flex items-center gap-3 rounded-full border border-white/10 bg-white/8 px-4 py-2 text-sm text-white">
                    <span>Loop</span>
                    <div className="h-6 w-11 rounded-full bg-white/10 p-1">
                      <div className="ml-auto h-4 w-4 rounded-full bg-white" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </GlassCard>

          <GlassCard className="p-4">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Frames</p>
                <p className="mt-1 text-sm text-white">Sequence strip</p>
              </div>
            </div>
            <div className="flex gap-3 overflow-x-auto pb-1">
              {spriteFrames.map((frame, index) => (
                <div key={frame.id} className="min-w-28 rounded-[20px] border border-white/8 bg-white/[0.04] p-2">
                  <Image
                    src={frame.preview}
                    alt={frame.id}
                    width={640}
                    height={480}
                    unoptimized
                    className="h-20 w-full rounded-[16px] object-cover"
                  />
                  <div className="mt-2 text-center text-xs text-zinc-500">{index + 1}</div>
                </div>
              ))}
            </div>
          </GlassCard>
        </div>
      </div>
    </div>
  );
}
