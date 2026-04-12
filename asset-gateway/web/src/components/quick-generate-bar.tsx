"use client";

import { motion } from "framer-motion";
import { ArrowUpRight } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { GlassCard } from "@/components/glass-card";
import { CATEGORY_META, STUDIO_KINDS, type AssetKind } from "@/lib/categories";
import { cn, studioPath } from "@/lib/utils";

export function QuickGenerateBar() {
  const router = useRouter();
  const [selected, setSelected] = useState<AssetKind>("image");
  const [prompt, setPrompt] = useState("Cinematic plaza, mirrored rain, soft cyan fog");

  return (
    <GlassCard className="overflow-hidden border-white/12 bg-zinc-950/70 p-4">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
        <div className="flex gap-2 overflow-x-auto pb-1">
          {STUDIO_KINDS.map((kind) => {
            const meta = CATEGORY_META[kind];
            const Icon = meta.icon;

            return (
              <motion.button
                key={kind}
                whileHover={{ y: -2 }}
                whileTap={{ scale: 0.98 }}
                onClick={() => setSelected(kind)}
                title={meta.label}
                className={cn(
                  "group relative flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl border transition",
                  selected === kind
                    ? "border-white/18 bg-white/12 text-white"
                    : "border-white/8 bg-white/[0.04] text-zinc-500 hover:text-white",
                )}
              >
                <Icon className={cn("h-5 w-5", meta.color)} />
                <span className="pointer-events-none absolute -bottom-8 rounded-full bg-black/70 px-2 py-1 text-[10px] uppercase tracking-[0.2em] text-white opacity-0 transition group-hover:opacity-100">
                  {meta.label}
                </span>
              </motion.button>
            );
          })}
        </div>

        <div className="flex flex-1 items-center gap-3 rounded-[26px] border border-white/10 bg-white/[0.04] p-2 pl-5">
          <input
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            className="h-14 flex-1 bg-transparent text-base text-white outline-none placeholder:text-zinc-500"
            placeholder="Describe the asset you want to conjure"
          />
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={() =>
              router.push(`${studioPath(selected)}?prompt=${encodeURIComponent(prompt)}&kind=${selected}`)
            }
            className="flex h-14 items-center gap-2 rounded-[22px] border border-white/12 bg-white px-5 text-sm font-semibold text-zinc-950 shadow-[0_16px_40px_rgba(255,255,255,0.12)] transition hover:bg-cyan-200"
          >
            Generate
            <ArrowUpRight className="h-4 w-4" />
          </motion.button>
        </div>
      </div>
    </GlassCard>
  );
}
