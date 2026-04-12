"use client";

import { AnimatePresence, motion } from "framer-motion";
import { Copy, Plus, ShieldCheck, X } from "lucide-react";
import { useState } from "react";
import { GlassCard } from "@/components/glass-card";
import { apiKeys, quota, usageBars } from "@/lib/mock-data";

export default function SettingsPage() {
  const [open, setOpen] = useState(false);

  return (
    <div className="space-y-5 pb-24">
      <GlassCard className="p-6">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">Workspace controls</p>
            <h1 className="mt-3 text-5xl leading-none">Keys, quota, signal.</h1>
          </div>
          <motion.button
            whileHover={{ y: -2 }}
            whileTap={{ scale: 0.98 }}
            onClick={() => setOpen(true)}
            className="inline-flex h-14 items-center gap-2 rounded-[22px] border border-white/12 bg-white px-5 text-sm font-semibold text-zinc-950"
          >
            <Plus className="h-4 w-4" />
            New key
          </motion.button>
        </div>
      </GlassCard>

      <div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
        <GlassCard id="keys" className="p-6">
          <div className="mb-5 flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">API keys</p>
              <p className="mt-2 text-xl text-white">Gateway access cards</p>
            </div>
            <ShieldCheck className="h-5 w-5 text-emerald-300" />
          </div>

          <div className="space-y-3">
            {apiKeys.map((key) => (
              <div
                key={key.prefix}
                className="flex items-center justify-between rounded-[24px] border border-white/8 bg-white/[0.04] p-4"
              >
                <div>
                  <div className="text-base font-medium text-white">{key.prefix}••••••••</div>
                  <div className="mt-1 text-xs uppercase tracking-[0.24em] text-zinc-500">
                    created {key.createdAt} · last used {key.lastUsed}
                  </div>
                </div>
                <div className="flex gap-2">
                  <button className="flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-white/8 text-white hover:bg-white/12">
                    <Copy className="h-4 w-4" />
                  </button>
                  <button className="rounded-2xl border border-white/10 bg-white/8 px-4 text-sm text-white hover:bg-white/12">
                    Revoke
                  </button>
                </div>
              </div>
            ))}
          </div>
        </GlassCard>

        <GlassCard className="p-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">Usage</p>
              <p className="mt-2 text-xl text-white">Quota rhythm</p>
            </div>
            <div className="text-right">
              <div className="text-3xl font-semibold text-white">{quota.used}</div>
              <div className="text-xs uppercase tracking-[0.24em] text-zinc-500">used / {quota.total}</div>
            </div>
          </div>

          <div className="mt-8 flex h-56 items-end gap-2">
            {usageBars.map((value, index) => (
              <motion.div
                key={`${value}-${index}`}
                initial={{ height: 0 }}
                animate={{ height: `${value}%` }}
                transition={{ delay: index * 0.03 }}
                className="flex-1 rounded-t-[18px] bg-[linear-gradient(180deg,rgba(34,211,238,0.9),rgba(236,72,153,0.45))]"
              />
            ))}
          </div>

          <div className="mt-5 grid grid-cols-3 gap-3 text-center">
            {[
              ["image", "124"],
              ["video", "29"],
              ["audio", "76"],
            ].map(([label, count]) => (
              <div key={label} className="rounded-[22px] border border-white/8 bg-white/[0.04] px-3 py-4">
                <div className="text-2xl font-semibold text-white">{count}</div>
                <div className="mt-1 text-xs uppercase tracking-[0.24em] text-zinc-500">{label}</div>
              </div>
            ))}
          </div>
        </GlassCard>
      </div>

      <AnimatePresence>
        {open ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 grid place-items-center bg-black/70 px-4 backdrop-blur-md"
          >
            <motion.div
              initial={{ opacity: 0, y: 24, scale: 0.96 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 24, scale: 0.96 }}
              className="w-full max-w-lg rounded-[32px] border border-white/10 bg-zinc-950/95 p-6 shadow-[0_24px_90px_rgba(0,0,0,0.5)]"
            >
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">Create key</p>
                  <h2 className="mt-2 text-3xl text-white">Fresh token</h2>
                </div>
                <button
                  onClick={() => setOpen(false)}
                  className="flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-white/8 text-white"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>

              <div className="mt-6 space-y-4">
                <label className="block space-y-2">
                  <span className="text-xs uppercase tracking-[0.24em] text-zinc-500">Label</span>
                  <input className="w-full rounded-[22px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none" defaultValue="design-bot" />
                </label>
                <label className="block space-y-2">
                  <span className="text-xs uppercase tracking-[0.24em] text-zinc-500">Quota</span>
                  <input className="w-full rounded-[22px] border border-white/10 bg-white/[0.04] px-4 py-4 text-white outline-none" defaultValue="100" />
                </label>
              </div>

              <button className="mt-6 flex h-14 w-full items-center justify-center rounded-[22px] border border-white/12 bg-white text-sm font-semibold text-zinc-950">
                Create key
              </button>
            </motion.div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
