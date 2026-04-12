"use client";

import { motion } from "framer-motion";
import {
  AudioLines,
  Box,
  Clapperboard,
  FileText,
  Fingerprint,
  Globe2,
  ImageIcon,
  Mic,
  Music4,
  SlidersHorizontal,
  Sparkles,
} from "lucide-react";
import GlowCard from "@/components/ui/glow-card";

const capabilities = [
  { label: "Image", icon: ImageIcon, gradient: "from-cyan-400/35 via-blue-500/12 to-indigo-500/18" },
  { label: "Video", icon: Clapperboard, gradient: "from-fuchsia-400/35 via-rose-500/12 to-orange-500/16" },
  { label: "Audio", icon: AudioLines, gradient: "from-emerald-400/35 via-lime-400/12 to-teal-500/18" },
  { label: "Music", icon: Music4, gradient: "from-violet-400/35 via-sky-500/12 to-cyan-400/18" },
  { label: "TTS", icon: Mic, gradient: "from-amber-400/35 via-orange-500/12 to-rose-400/16" },
  { label: "Voice", icon: Fingerprint, gradient: "from-pink-400/32 via-fuchsia-500/14 to-violet-500/18" },
  { label: "3D Model", icon: Box, gradient: "from-sky-400/35 via-cyan-500/12 to-emerald-500/18" },
  { label: "Sprite", icon: Sparkles, gradient: "from-lime-300/30 via-emerald-400/14 to-cyan-400/16" },
  { label: "3D World", icon: Globe2, gradient: "from-blue-400/30 via-violet-500/12 to-fuchsia-400/20" },
  { label: "Text", icon: FileText, gradient: "from-zinc-200/20 via-cyan-400/10 to-zinc-500/18" },
  { label: "Process", icon: SlidersHorizontal, gradient: "from-orange-300/30 via-amber-400/14 to-yellow-400/16" },
];

export default function CapabilityGrid() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between gap-4">
        <h2 className="text-xl font-semibold tracking-[0.22em] text-zinc-400 uppercase">
          Capabilities
        </h2>
        <p className="hidden text-sm text-zinc-500 sm:block">Visual-first surfaces for every mode.</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {capabilities.map((capability, index) => {
          const Icon = capability.icon;
          return (
            <motion.div
              key={capability.label}
              initial={{ opacity: 0, y: 28 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, amount: 0.2 }}
              transition={{ duration: 0.45, delay: index * 0.05 }}
            >
              <GlowCard glowClassName={capability.gradient} className="h-full min-h-[18rem]">
                <div className="flex h-full flex-col p-5">
                  <div className={`relative mb-5 flex-1 overflow-hidden rounded-[1.5rem] border border-white/10 bg-gradient-to-br ${capability.gradient}`}>
                    <div className="absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(255,255,255,0.26),transparent_26%),linear-gradient(180deg,rgba(255,255,255,0.08),transparent_50%)]" />
                    <div className="absolute inset-x-5 top-5 flex items-center justify-between text-zinc-100">
                      <Icon className="h-6 w-6" />
                      <div className="h-2 w-14 rounded-full bg-white/30" />
                    </div>
                    <div className="absolute bottom-5 left-5 right-5 grid grid-cols-3 gap-2">
                      <div className="h-12 rounded-2xl bg-black/20 backdrop-blur-xl" />
                      <div className="h-[4.5rem] rounded-2xl bg-black/15 backdrop-blur-xl" />
                      <div className="h-10 rounded-2xl bg-black/20 backdrop-blur-xl" />
                    </div>
                  </div>
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-lg font-semibold text-white">{capability.label}</div>
                    <Icon className="h-4 w-4 text-zinc-400 transition duration-300 group-hover:text-cyan-200" />
                  </div>
                </div>
              </GlowCard>
            </motion.div>
          );
        })}
      </div>
    </div>
  );
}
