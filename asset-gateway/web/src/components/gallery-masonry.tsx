"use client";

import { motion } from "framer-motion";

const galleryItems = [
  { rows: 20, gradient: "from-cyan-500/30 via-blue-500/14 to-zinc-950", accent: "Output 01" },
  { rows: 28, gradient: "from-fuchsia-500/30 via-rose-500/14 to-zinc-950", accent: "Output 02" },
  { rows: 18, gradient: "from-emerald-500/30 via-teal-500/14 to-zinc-950", accent: "Output 03" },
  { rows: 26, gradient: "from-amber-400/30 via-orange-500/14 to-zinc-950", accent: "Output 04" },
  { rows: 16, gradient: "from-violet-500/30 via-sky-500/14 to-zinc-950", accent: "Output 05" },
  { rows: 24, gradient: "from-lime-400/28 via-emerald-500/14 to-zinc-950", accent: "Output 06" },
  { rows: 22, gradient: "from-pink-500/28 via-fuchsia-500/14 to-zinc-950", accent: "Output 07" },
  { rows: 30, gradient: "from-sky-500/30 via-cyan-500/14 to-zinc-950", accent: "Output 08" },
];

export default function GalleryMasonry() {
  return (
    <div className="space-y-8">
      <h2 className="text-4xl font-bold tracking-[-0.05em] text-white">Gallery</h2>
      <div className="gallery-grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {galleryItems.map((item, index) => (
          <motion.div
            key={item.accent}
            initial={{ opacity: 0, y: 30 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, amount: 0.12 }}
            transition={{ duration: 0.5, delay: index * 0.06 }}
            style={{ gridRow: `span ${item.rows}` }}
            className="gallery-card group relative overflow-hidden rounded-[2rem] border border-white/10 bg-zinc-950/72 p-4"
          >
            <div className={`absolute inset-0 bg-gradient-to-br ${item.gradient}`} />
            <div className="absolute inset-[10%] rounded-[1.75rem] border border-white/10 bg-black/12 backdrop-blur-2xl transition duration-500 group-hover:scale-[1.03]" />
            <div className="absolute inset-x-4 top-4 flex items-center justify-between text-xs font-medium uppercase tracking-[0.24em] text-zinc-100">
              <span>{item.accent}</span>
              <span>{String(index + 1).padStart(2, "0")}</span>
            </div>
            <div className="absolute bottom-4 left-4 right-4 grid grid-cols-3 gap-2">
              <div className="h-12 rounded-2xl bg-white/10 backdrop-blur-xl" />
              <div className="h-16 rounded-2xl bg-white/12 backdrop-blur-xl" />
              <div className="h-10 rounded-2xl bg-white/10 backdrop-blur-xl" />
            </div>
          </motion.div>
        ))}
      </div>
    </div>
  );
}
