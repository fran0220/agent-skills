"use client";

import { motion } from "framer-motion";
import { ImagePlus, Layers2, Sparkles, Wand2 } from "lucide-react";
import Image from "next/image";
import { GlassCard } from "@/components/glass-card";
import { sessionFrames } from "@/lib/mock-data";
import { cn } from "@/lib/utils";

const editModes = ["generate", "edit", "inpaint", "restyle", "expand"] as const;
const styles = ["Editorial", "Soft 3D", "Ink", "Glass", "Pixel Glow"];

export function ImageStudioView({ prompt }: { prompt: string }) {
  return (
    <div className="min-h-screen bg-zinc-950 px-4 py-5 text-white lg:px-6">
      <div className="grid gap-5 xl:grid-cols-[340px_minmax(0,1fr)_320px]">
        <GlassCard className="p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.3em] text-zinc-500">Image Studio</p>
              <h1 className="mt-3 text-4xl leading-none">Frame & refine</h1>
            </div>
            <Sparkles className="h-5 w-5 text-cyan-300" />
          </div>

          <div className="mt-6 space-y-3">
            <label className="block space-y-2">
              <span className="text-xs uppercase tracking-[0.24em] text-zinc-500">Prompt</span>
              <textarea
                defaultValue={prompt}
                className="min-h-40 w-full rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none"
              />
            </label>

            <div className="space-y-2">
              <span className="text-xs uppercase tracking-[0.24em] text-zinc-500">Mode</span>
              <div className="grid grid-cols-2 gap-2">
                {editModes.map((mode) => (
                  <button
                    key={mode}
                    className={cn(
                      "rounded-[22px] border px-4 py-3 text-sm capitalize transition",
                      mode === "restyle"
                        ? "border-cyan-400/30 bg-cyan-400/12 text-white"
                        : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                    )}
                  >
                    {mode}
                  </button>
                ))}
              </div>
            </div>

            <label className="grid min-h-44 place-items-center rounded-[28px] border border-dashed border-white/16 bg-[linear-gradient(180deg,rgba(34,211,238,0.12),transparent_60%)] p-6 text-center">
              <input type="file" className="hidden" />
              <div>
                <ImagePlus className="mx-auto h-8 w-8 text-cyan-300" />
                <p className="mt-4 text-sm text-white">Drop reference image</p>
                <p className="mt-2 text-xs uppercase tracking-[0.24em] text-zinc-500">drag / paste / upload</p>
              </div>
            </label>
          </div>
        </GlassCard>

        <div className="space-y-5">
          <GlassCard className="relative min-h-[620px] overflow-hidden p-4">
            <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(34,211,238,0.18),_transparent_44%),linear-gradient(180deg,rgba(24,24,27,0.1),rgba(9,9,11,0.82))]" />
            <div className="relative flex h-full flex-col rounded-[28px] border border-white/8 bg-zinc-950/50">
              <div className="flex items-center justify-between border-b border-white/8 px-5 py-4">
                <div>
                  <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Canvas</p>
                  <p className="mt-1 text-sm text-white">1792 × 1024 · transparent off</p>
                </div>
                <button className="rounded-2xl border border-white/10 bg-white/8 px-4 py-2 text-sm text-white">
                  Regenerate
                </button>
              </div>

              <div className="grid flex-1 place-items-center p-8">
                <motion.div
                  animate={{ scale: [1, 1.02, 1] }}
                  transition={{ duration: 6, repeat: Number.POSITIVE_INFINITY }}
                  className="relative aspect-[16/10] w-full max-w-4xl overflow-hidden rounded-[32px] border border-white/10 bg-[radial-gradient(circle_at_20%_20%,rgba(34,211,238,0.2),transparent_20%),linear-gradient(145deg,#083344,#111827_48%,#09090b)]"
                >
                  <div className="absolute inset-0 bg-[linear-gradient(135deg,rgba(255,255,255,0.18),transparent_38%)]" />
                  <div className="absolute inset-x-8 bottom-8 rounded-[28px] border border-white/10 bg-black/30 p-5 backdrop-blur-lg">
                    <p className="text-xs uppercase tracking-[0.3em] text-cyan-200/60">Preview</p>
                    <p className="mt-3 max-w-[24ch] text-3xl leading-tight text-white">Cinematic atmosphere with layered glass reflections.</p>
                  </div>
                </motion.div>
              </div>
            </div>
          </GlassCard>

          <GlassCard className="p-4">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Session</p>
                <p className="mt-1 text-sm text-white">Edit history</p>
              </div>
              <Layers2 className="h-4 w-4 text-cyan-300" />
            </div>
            <div className="flex gap-3 overflow-x-auto pb-1">
              {sessionFrames.map((frame, index) => (
                <div key={frame.id} className="min-w-36 rounded-[22px] border border-white/8 bg-white/[0.04] p-2">
                  <Image
                    src={frame.preview}
                    alt={frame.label}
                    width={640}
                    height={480}
                    unoptimized
                    className="h-24 w-full rounded-[18px] object-cover"
                  />
                  <div className="mt-2 flex items-center justify-between px-1 text-xs text-zinc-400">
                    <span>{frame.label}</span>
                    <span>{index === sessionFrames.length - 1 ? "live" : "saved"}</span>
                  </div>
                </div>
              ))}
            </div>
          </GlassCard>
        </div>

        <GlassCard className="p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Controls</p>
              <p className="mt-2 text-xl text-white">Dial in the render</p>
            </div>
            <Wand2 className="h-5 w-5 text-cyan-300" />
          </div>

          <div className="mt-6 space-y-5">
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm text-zinc-400">
                <span>Canvas size</span>
                <span className="text-white">1792 × 1024</span>
              </div>
              <input type="range" defaultValue={82} className="w-full accent-cyan-300" />
            </div>

            <div className="flex items-center justify-between rounded-[22px] border border-white/8 bg-white/[0.04] px-4 py-4">
              <div>
                <p className="text-sm text-white">Transparent background</p>
                <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">gpt image priority</p>
              </div>
              <div className="h-8 w-14 rounded-full bg-white/10 p-1">
                <div className="h-6 w-6 rounded-full bg-white" />
              </div>
            </div>

            <div className="space-y-2">
              <span className="text-xs uppercase tracking-[0.24em] text-zinc-500">Style cards</span>
              <div className="grid gap-2">
                {styles.map((style, index) => (
                  <button
                    key={style}
                    className={cn(
                      "rounded-[22px] border px-4 py-4 text-left transition",
                      index === 1
                        ? "border-cyan-400/30 bg-cyan-400/12 text-white"
                        : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                    )}
                  >
                    {style}
                  </button>
                ))}
              </div>
            </div>

            <button className="w-full rounded-[24px] border border-white/12 bg-white px-4 py-4 text-sm font-semibold text-zinc-950">
              Generate image
            </button>
          </div>
        </GlassCard>
      </div>
    </div>
  );
}
