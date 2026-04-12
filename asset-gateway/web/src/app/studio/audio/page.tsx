"use client";

import { motion } from "framer-motion";
import { AudioLines, Mic2, Music4, Sparkles } from "lucide-react";
import { useState } from "react";
import { AudioWaveform } from "@/components/audio-waveform";
import { GlassCard } from "@/components/glass-card";
import { audioHistory } from "@/lib/mock-data";
import { cn } from "@/lib/utils";

const audioModes = [
  { id: "audio", icon: AudioLines, label: "SFX", tint: "text-amber-300" },
  { id: "music", icon: Music4, label: "BGM", tint: "text-pink-300" },
  { id: "tts", icon: Mic2, label: "Voice", tint: "text-emerald-300" },
] as const;

export default function AudioLabPage() {
  const [mode, setMode] = useState<(typeof audioModes)[number]["id"]>("audio");

  return (
    <div className="min-h-screen bg-zinc-950 px-4 py-5 text-white lg:px-6">
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-5">
          <GlassCard className="p-5">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-zinc-500">Audio Lab</p>
                <h1 className="mt-3 text-4xl leading-none">Shape the waveform</h1>
              </div>
              <Sparkles className="h-5 w-5 text-amber-300" />
            </div>

            <div className="mt-6 rounded-[28px] border border-white/8 bg-[linear-gradient(180deg,rgba(251,191,36,0.1),rgba(9,9,11,0.4))] p-6">
              <AudioWaveform className="w-full" height={180} />
            </div>

            <div className="mt-5 grid gap-3 md:grid-cols-3">
              {audioModes.map((item) => {
                const Icon = item.icon;
                const active = item.id === mode;
                return (
                  <motion.button
                    key={item.id}
                    whileHover={{ y: -4 }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => setMode(item.id)}
                    className={cn(
                      "rounded-[24px] border p-5 text-left transition",
                      active
                        ? "border-white/16 bg-white/10 text-white"
                        : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                    )}
                  >
                    <Icon className={cn("h-6 w-6", item.tint)} />
                    <p className="mt-4 text-lg font-medium">{item.label}</p>
                  </motion.button>
                );
              })}
            </div>
          </GlassCard>

          <GlassCard className="p-5">
            <div className="space-y-4">
              <textarea
                defaultValue="Elastic mechanical impacts with warm sub-bass tail and shimmering metal dust"
                className="min-h-36 w-full rounded-[24px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none"
              />
              <div className="grid gap-4 md:grid-cols-2">
                {[
                  ["Duration", "14s", 52],
                  [mode === "tts" ? "Expressiveness" : "Energy", "0.68", 68],
                ].map(([label, value, progress]) => (
                  <div key={label} className="space-y-2">
                    <div className="flex items-center justify-between text-sm text-zinc-400">
                      <span>{label}</span>
                      <span className="text-white">{value}</span>
                    </div>
                    <input type="range" defaultValue={progress} className="w-full accent-amber-300" />
                  </div>
                ))}
              </div>
              <button className="rounded-[24px] border border-white/12 bg-white px-5 py-4 text-sm font-semibold text-zinc-950">
                Generate audio
              </button>
            </div>
          </GlassCard>

          <GlassCard className="p-5">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">History</p>
                <p className="mt-1 text-sm text-white">Recent takes</p>
              </div>
            </div>
            <div className="flex gap-3 overflow-x-auto pb-1">
              {audioHistory.map((item) => (
                <div
                  key={item.id}
                  className="min-w-48 rounded-[22px] border border-white/8 bg-white/[0.04] p-4"
                >
                  <div className="flex items-center justify-between text-sm text-white">
                    <span>{item.title}</span>
                    <span className="text-zinc-500">{item.kind}</span>
                  </div>
                  <AudioWaveform className="mt-5" height={80} />
                </div>
              ))}
            </div>
          </GlassCard>
        </div>

        <GlassCard className="p-5">
          <div>
            <p className="text-xs uppercase tracking-[0.24em] text-zinc-500">Preset tone</p>
            <p className="mt-2 text-xl text-white">Signal sculpt</p>
          </div>
          <div className="mt-6 space-y-3">
            {["Dry", "Wide", "Noisy", "Clean", "Cinematic"].map((preset, index) => (
              <button
                key={preset}
                className={cn(
                  "w-full rounded-[22px] border px-4 py-4 text-left transition",
                  index === 2
                    ? "border-amber-400/30 bg-amber-300/10 text-white"
                    : "border-white/8 bg-white/[0.04] text-zinc-400 hover:text-white",
                )}
              >
                {preset}
              </button>
            ))}
          </div>
        </GlassCard>
      </div>
    </div>
  );
}
