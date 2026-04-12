"use client";

import Image from "next/image";
import { motion } from "framer-motion";

export default function HeroSection() {
  return (
    <section className="space-y-8">
      <motion.div
        initial={{ opacity: 0, scale: 0.97, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
        className="relative overflow-hidden rounded-[2rem] border border-white/10"
      >
        <Image
          src="/showcase/hero-floating-islands.png"
          alt="Cinematic floating islands generated with AssetForge"
          width={1792}
          height={1024}
          unoptimized
          priority
          className="w-full"
        />
        <div className="absolute left-4 top-4 flex items-center gap-2 rounded-full border border-white/12 bg-zinc-950/55 px-3 py-1.5 text-[11px] font-medium uppercase tracking-[0.22em] text-zinc-200 backdrop-blur-xl sm:left-6 sm:top-6">
          <span className="h-1.5 w-1.5 rounded-full bg-cyan-400" />
          Generated with AssetForge
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay: 0.3 }}
        className="flex flex-col items-center gap-6 text-center"
      >
        <div className="flex flex-wrap justify-center gap-3 text-sm text-zinc-400">
          {["Image", "Video", "Audio", "3D", "Sprite", "World"].map((cat) => (
            <span key={cat} className="rounded-full border border-white/8 bg-white/[0.03] px-3 py-1">
              {cat}
            </span>
          ))}
        </div>
        <a
          href="/dashboard"
          className="group relative inline-flex items-center gap-2 rounded-full bg-cyan-400/14 px-6 py-3 text-sm font-semibold text-cyan-100 shadow-[0_0_40px_-12px_rgba(34,211,238,0.6)] transition hover:bg-cyan-300/20 hover:shadow-[0_0_60px_-12px_rgba(34,211,238,0.8)]"
        >
          Start Creating
          <span className="transition group-hover:translate-x-0.5">→</span>
        </a>
      </motion.div>
    </section>
  );
}
