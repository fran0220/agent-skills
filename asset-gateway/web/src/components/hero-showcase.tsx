"use client";

import { motion } from "framer-motion";
import {
  AudioLines,
  Box,
  Clapperboard,
  Globe2,
  ImageIcon,
  Sparkles,
} from "lucide-react";

const floatingCards = [
  {
    title: "Image",
    icon: ImageIcon,
    className: "left-4 top-6 sm:left-8 sm:top-8",
    gradient: "from-cyan-400/25 to-blue-500/10",
  },
  {
    title: "Video",
    icon: Clapperboard,
    className: "right-4 top-12 sm:right-8 sm:top-14",
    gradient: "from-fuchsia-400/25 to-rose-500/10",
  },
  {
    title: "Audio",
    icon: AudioLines,
    className: "left-8 bottom-8 sm:left-14 sm:bottom-10",
    gradient: "from-emerald-400/25 to-lime-500/10",
  },
  {
    title: "World",
    icon: Globe2,
    className: "right-6 bottom-10 sm:right-10 sm:bottom-12",
    gradient: "from-amber-400/25 to-orange-500/10",
  },
];

export default function HeroShowcase() {
  return (
    <motion.section
      initial={{ opacity: 0, scale: 0.96, y: 24 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] }}
      className="glass-panel relative overflow-hidden rounded-[2.25rem] px-4 py-6 shadow-[0_30px_120px_-55px_rgba(168,85,247,0.65)] sm:px-6 lg:px-8"
    >
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.11),transparent_26%),radial-gradient(circle_at_20%_85%,rgba(34,211,238,0.18),transparent_26%),radial-gradient(circle_at_80%_20%,rgba(168,85,247,0.2),transparent_28%)]" />

      <div className="relative grid gap-4 lg:grid-cols-[1.3fr_0.7fr]">
        <div className="glow-mesh relative min-h-[22rem] overflow-hidden rounded-[2rem] border border-white/10 p-5 sm:min-h-[28rem] sm:p-8">
          <motion.div
            animate={{ y: [0, -8, 0], rotate: [0, 1.2, 0] }}
            transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
            className="absolute inset-x-[16%] top-[14%] h-[58%] rounded-[2rem] border border-white/14 bg-zinc-950/45 shadow-[0_25px_90px_-55px_rgba(34,211,238,0.95)] backdrop-blur-xl"
          >
            <div className="flex h-full flex-col justify-between p-5">
              <div className="flex items-center justify-between text-xs uppercase tracking-[0.26em] text-zinc-300">
                <span>Rendering</span>
                <span>87%</span>
              </div>
              <div className="space-y-4">
                <div className="h-36 rounded-[1.5rem] bg-[radial-gradient(circle_at_25%_25%,rgba(34,211,238,0.65),transparent_20%),radial-gradient(circle_at_80%_20%,rgba(168,85,247,0.55),transparent_18%),radial-gradient(circle_at_55%_80%,rgba(16,185,129,0.35),transparent_24%),linear-gradient(135deg,rgba(39,39,42,0.85),rgba(9,9,11,0.98))]" />
                <div className="space-y-2">
                  <div className="h-2 rounded-full bg-white/10">
                    <motion.div
                      className="h-full rounded-full bg-gradient-to-r from-cyan-300 via-sky-400 to-violet-400"
                      initial={{ width: "24%" }}
                      animate={{ width: ["24%", "91%", "72%", "87%"] }}
                      transition={{ duration: 6, repeat: Infinity, ease: "easeInOut" }}
                    />
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    <div className="h-14 rounded-2xl bg-white/8" />
                    <div className="h-14 rounded-2xl bg-white/[0.06]" />
                    <div className="h-14 rounded-2xl bg-white/10" />
                  </div>
                </div>
              </div>
            </div>
          </motion.div>

          {floatingCards.map((card, index) => {
            const Icon = card.icon;
            return (
              <motion.div
                key={card.title}
                initial={{ opacity: 0, y: 24 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 + index * 0.12, duration: 0.6 }}
                whileHover={{ scale: 1.04, rotate: 0 }}
                className={`absolute ${card.className} w-28 rounded-[1.5rem] border border-white/12 bg-zinc-950/55 p-3 backdrop-blur-xl sm:w-36`}
              >
                <div className={`mb-8 h-20 rounded-[1.15rem] bg-gradient-to-br ${card.gradient}`} />
                <div className="flex items-center gap-2 text-sm text-white">
                  <Icon className="h-4 w-4 text-zinc-300" />
                  <span>{card.title}</span>
                </div>
              </motion.div>
            );
          })}

          <div className="absolute left-1/2 top-6 flex -translate-x-1/2 items-center gap-2 rounded-full border border-white/10 bg-zinc-950/55 px-3 py-1.5 text-[11px] font-medium uppercase tracking-[0.22em] text-zinc-300 backdrop-blur-xl">
            <Sparkles className="h-3.5 w-3.5 text-cyan-200" />
            Live Pipeline
          </div>
        </div>

        <div className="grid gap-4">
          <motion.div
            initial={{ opacity: 0, x: 18 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.25, duration: 0.55 }}
            className="rounded-[2rem] border border-white/10 bg-zinc-950/62 p-5"
          >
            <div className="mb-4 flex items-center justify-between text-xs uppercase tracking-[0.26em] text-zinc-500">
              <span>Output Set</span>
              <Box className="h-4 w-4 text-zinc-400" />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="aspect-square rounded-[1.4rem] bg-[linear-gradient(145deg,rgba(34,211,238,0.2),rgba(9,9,11,0.85))]" />
              <div className="aspect-[0.9] rounded-[1.4rem] bg-[linear-gradient(145deg,rgba(251,191,36,0.24),rgba(9,9,11,0.82))]" />
              <div className="col-span-2 h-28 rounded-[1.4rem] bg-[linear-gradient(145deg,rgba(168,85,247,0.24),rgba(9,9,11,0.92))]" />
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 18 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.35, duration: 0.55 }}
            className="rounded-[2rem] border border-white/10 bg-zinc-950/62 p-5"
          >
            <div className="mb-4 flex items-center justify-between text-xs uppercase tracking-[0.26em] text-zinc-500">
              <span>Queued Modes</span>
              <span className="text-zinc-300">11</span>
            </div>
            <div className="flex flex-wrap gap-2">
              {[
                "Image",
                "Video",
                "Audio",
                "Music",
                "TTS",
                "Voice",
                "3D",
                "Sprite",
                "World",
                "Text",
                "Process",
              ].map((item) => (
                <span
                  key={item}
                  className="rounded-full border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-zinc-300"
                >
                  {item}
                </span>
              ))}
            </div>
          </motion.div>
        </div>
      </div>
    </motion.section>
  );
}
