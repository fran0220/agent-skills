"use client";

import { motion } from "framer-motion";
import { Download, Heart, Play, Rotate3D, Share2, Trash2 } from "lucide-react";
import Image from "next/image";
import { AudioWaveform } from "@/components/audio-waveform";
import { GlassCard } from "@/components/glass-card";
import { CATEGORY_META, type AssetKind } from "@/lib/categories";
import { cn } from "@/lib/utils";

type AssetCardProps = {
  asset: {
    id: string;
    kind: AssetKind;
    title: string;
    prompt: string;
    createdAt: string;
    preview: string;
    height: number;
    author?: string;
    likes?: number;
  };
  publicMode?: boolean;
};

export function AssetCard({ asset, publicMode = false }: AssetCardProps) {
  const meta = CATEGORY_META[asset.kind];
  const Icon = meta.icon;

  return (
    <motion.div layout whileHover={{ y: -8, scale: 1.01 }} transition={{ duration: 0.22 }}>
      <GlassCard
        className={cn(
          "group relative mb-5 overflow-hidden border-white/8 bg-zinc-900/55",
          meta.glow,
          "break-inside-avoid",
        )}
      >
        <div
          className={cn(
            "absolute inset-x-0 top-0 h-28 bg-gradient-to-br opacity-80",
            meta.soft,
          )}
        />

        <div className="relative p-3">
          <div
            className="relative overflow-hidden rounded-[22px] border border-white/10 bg-zinc-950/80"
            style={{ minHeight: asset.height }}
          >
            {(asset.kind === "image" ||
              asset.kind === "video" ||
              asset.kind === "sprite" ||
              asset.kind === "world" ||
              asset.kind === "model3d") && (
              <Image
                src={asset.preview}
                alt={asset.title}
                fill
                unoptimized
                sizes="(max-width: 768px) 100vw, (max-width: 1536px) 50vw, 33vw"
                className="object-cover saturate-[1.08]"
              />
            )}

            {(asset.kind === "audio" || asset.kind === "music" || asset.kind === "tts") && (
              <div className="flex h-full min-h-[220px] flex-col justify-between bg-[radial-gradient(circle_at_top,_rgba(255,255,255,0.12),_transparent_55%)] p-5">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs uppercase tracking-[0.32em] text-white/45">listen</p>
                    <h4 className="mt-2 text-lg font-semibold text-white">{asset.title}</h4>
                  </div>
                  <button className="flex h-11 w-11 items-center justify-center rounded-full border border-white/12 bg-white/10 text-white transition hover:bg-white/16">
                    <Play className="h-4 w-4" fill="currentColor" />
                  </button>
                </div>
                <AudioWaveform className="mt-8" height={88} />
              </div>
            )}

            {asset.kind === "text" && (
              <div className="flex h-full min-h-[220px] flex-col justify-between bg-[linear-gradient(160deg,rgba(255,255,255,0.12),transparent_42%),linear-gradient(180deg,#111114_0%,#050507_100%)] p-6">
                <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">narrative seed</p>
                <p className="max-w-[18ch] text-2xl font-semibold leading-tight text-white">
                  {asset.prompt}
                </p>
                <div className="h-px w-full bg-white/8" />
              </div>
            )}

            {asset.kind === "video" && (
              <div className="absolute inset-0 flex items-center justify-center bg-black/16">
                <div className="flex h-16 w-16 items-center justify-center rounded-full border border-white/12 bg-black/35 backdrop-blur-md">
                  <Play className="h-6 w-6 text-white" fill="currentColor" />
                </div>
              </div>
            )}

            {asset.kind === "model3d" && (
              <div className="absolute right-5 top-5 flex h-12 w-12 items-center justify-center rounded-full border border-white/12 bg-zinc-950/60 text-white/80 backdrop-blur-md">
                <Rotate3D className="h-5 w-5 animate-[spin_7s_linear_infinite]" />
              </div>
            )}

            <div className="absolute inset-x-3 bottom-3 translate-y-3 rounded-2xl border border-white/10 bg-black/42 p-3 opacity-0 backdrop-blur-xl transition duration-300 group-hover:translate-y-0 group-hover:opacity-100">
              <div className="mb-2 flex items-center justify-between text-xs text-white/55">
                <span>{asset.title}</span>
                {publicMode && asset.author ? <span>@{asset.author}</span> : null}
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  {publicMode ? (
                    <button className="flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/8 text-white transition hover:bg-white/14">
                      <Heart className="h-4 w-4" />
                    </button>
                  ) : (
                    <>
                      <button className="flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/8 text-white transition hover:bg-white/14">
                        <Download className="h-4 w-4" />
                      </button>
                      <button className="flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/8 text-white transition hover:bg-white/14">
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </>
                  )}
                </div>
                <button className="flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/8 text-white transition hover:bg-white/14">
                  <Share2 className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>

          <div className="flex items-center justify-between px-2 pb-1 pt-4">
            <div className="flex items-center gap-3">
              <div
                className={cn(
                  "flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-white/5",
                  meta.color,
                )}
              >
                <Icon className="h-4 w-4" />
              </div>
              <div>
                <p className="text-sm font-medium text-white">{asset.title}</p>
                <p className="text-xs text-zinc-500">{asset.prompt}</p>
              </div>
            </div>
            <div className="text-right text-xs text-zinc-500">
              <div>{asset.createdAt}</div>
              {publicMode && typeof asset.likes === "number" ? <div>{asset.likes} likes</div> : null}
            </div>
          </div>
        </div>
      </GlassCard>
    </motion.div>
  );
}
