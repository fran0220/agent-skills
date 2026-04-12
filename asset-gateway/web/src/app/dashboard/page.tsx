"use client";

import { motion } from "framer-motion";
import Link from "next/link";
import { ArrowRight, Orbit, Sparkles } from "lucide-react";
import { AssetCard } from "@/components/asset-card";
import { GlassCard } from "@/components/glass-card";
import { QuickGenerateBar } from "@/components/quick-generate-bar";
import { CATEGORY_META, DASHBOARD_KINDS } from "@/lib/categories";
import { dashboardAssets, quota } from "@/lib/mock-data";
import { formatQuota, studioPath } from "@/lib/utils";

export default function DashboardPage() {
  const { remaining, percentage } = formatQuota(quota.total, quota.used);
  const circumference = 2 * Math.PI * 54;
  const dashOffset = circumference - (percentage / 100) * circumference;

  return (
    <div className="space-y-5 pb-24">
      <motion.section
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="grid gap-5 xl:grid-cols-[92px_minmax(0,1fr)_360px]"
      >
        <GlassCard className="hidden min-h-[520px] items-center justify-center border-white/8 bg-zinc-950/60 p-3 xl:flex">
          <div className="flex flex-col gap-3">
            {DASHBOARD_KINDS.map((kind) => {
              const meta = CATEGORY_META[kind];
              const Icon = meta.icon;
              return (
                <Link
                  key={kind}
                  href={studioPath(kind)}
                  title={meta.label}
                  className="group relative flex h-14 w-14 items-center justify-center rounded-[22px] border border-white/8 bg-white/[0.04] transition hover:-translate-y-1 hover:border-white/16"
                >
                  <Icon className={`h-5 w-5 ${meta.color}`} />
                  <span className="pointer-events-none absolute left-[74px] rounded-full bg-black/80 px-3 py-1 text-[10px] uppercase tracking-[0.2em] text-white opacity-0 transition group-hover:opacity-100">
                    {meta.label}
                  </span>
                </Link>
              );
            })}
          </div>
        </GlassCard>

        <div className="space-y-5">
          <GlassCard className="overflow-hidden border-white/10 bg-[linear-gradient(120deg,rgba(255,255,255,0.08),transparent_36%),linear-gradient(180deg,rgba(9,9,11,0.6),rgba(9,9,11,0.84))] p-6">
            <div className="mb-6 flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.34em] text-cyan-300/70">Control Deck</p>
                <h1 className="mt-3 max-w-[10ch] text-5xl leading-none md:text-6xl">
                  Build by seeing.
                </h1>
              </div>
              <div className="max-w-sm text-sm text-zinc-400">
                One feed for image, video, sprite, audio, world and 3D output. The prompt bar is your launch rail.
              </div>
            </div>
            <QuickGenerateBar />
          </GlassCard>

          <div className="columns-1 gap-5 md:columns-2 2xl:columns-3">
            {dashboardAssets.map((asset) => (
              <AssetCard key={asset.id} asset={asset} />
            ))}
          </div>
        </div>

        <div className="space-y-5">
          <GlassCard className="p-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">quota</p>
                <p className="mt-2 text-sm text-zinc-300">Remaining renders</p>
              </div>
              <Sparkles className="h-5 w-5 text-cyan-300" />
            </div>

            <div className="mt-6 flex items-center gap-5">
              <div className="relative flex h-36 w-36 items-center justify-center">
                <svg className="absolute inset-0 -rotate-90" viewBox="0 0 140 140">
                  <circle cx="70" cy="70" r="54" stroke="rgba(255,255,255,0.08)" strokeWidth="12" fill="none" />
                  <circle
                    cx="70"
                    cy="70"
                    r="54"
                    stroke="url(#quota-gradient)"
                    strokeWidth="12"
                    fill="none"
                    strokeLinecap="round"
                    strokeDasharray={circumference}
                    strokeDashoffset={dashOffset}
                  />
                  <defs>
                    <linearGradient id="quota-gradient" x1="0" y1="0" x2="1" y2="1">
                      <stop offset="0%" stopColor="#67e8f9" />
                      <stop offset="100%" stopColor="#f472b6" />
                    </linearGradient>
                  </defs>
                </svg>
                <div className="text-center">
                  <div className="text-4xl font-semibold text-white">{remaining}</div>
                  <div className="text-xs uppercase tracking-[0.28em] text-zinc-500">left</div>
                </div>
              </div>

              <div className="space-y-3 text-sm text-zinc-400">
                <div>
                  <p className="text-zinc-500">Used</p>
                  <p className="text-2xl font-semibold text-white">{quota.used}</p>
                </div>
                <div>
                  <p className="text-zinc-500">Total</p>
                  <p className="text-2xl font-semibold text-white">{quota.total}</p>
                </div>
              </div>
            </div>
          </GlassCard>

          <GlassCard className="p-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">Hot studio</p>
                <p className="mt-2 text-xl text-white">Image Session Stack</p>
              </div>
              <Orbit className="h-5 w-5 text-cyan-300" />
            </div>

            <div className="mt-5 space-y-3">
              {[
                ["Image", "cyan"],
                ["Video", "purple"],
                ["Sprite", "orange"],
              ].map(([label], index) => (
                <div key={label} className="rounded-[22px] border border-white/8 bg-white/[0.04] p-4">
                  <div className="flex items-center justify-between text-sm text-white">
                    <span>{label}</span>
                    <span className="text-zinc-500">0{index + 2} queued</span>
                  </div>
                  <div className="mt-3 h-2 overflow-hidden rounded-full bg-white/8">
                    <div className="h-full rounded-full bg-white" style={{ width: `${54 + index * 13}%` }} />
                  </div>
                </div>
              ))}
            </div>

            <Link
              href="/dashboard/gallery"
              className="mt-5 flex items-center justify-between rounded-[22px] border border-white/8 bg-white/[0.04] px-4 py-3 text-sm text-white transition hover:bg-white/[0.08]"
            >
              See all history
              <ArrowRight className="h-4 w-4" />
            </Link>
          </GlassCard>
        </div>
      </motion.section>
    </div>
  );
}
