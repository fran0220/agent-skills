"use client";

import { motion } from "framer-motion";
import { Check, X } from "lucide-react";
import GlowCard from "@/components/ui/glow-card";

const plans = [
  {
    name: "Free",
    price: "$0",
    unit: "/mo",
    highlight: false,
    glow: "from-zinc-300/16 via-cyan-400/8 to-zinc-600/18",
    features: [
      [true, "Preview modes"],
      [true, "Saved prompts"],
      [false, "High-res queue"],
      [false, "Team seats"],
    ] as const,
  },
  {
    name: "Pro",
    price: "$29",
    unit: "/mo",
    highlight: true,
    glow: "from-cyan-400/28 via-violet-400/18 to-fuchsia-400/22",
    features: [
      [true, "All 11 categories"],
      [true, "Priority runs"],
      [true, "Sessions + history"],
      [false, "Shared workspace"],
    ] as const,
  },
  {
    name: "Team",
    price: "$99",
    unit: "/mo",
    highlight: false,
    glow: "from-emerald-400/24 via-cyan-400/12 to-blue-500/18",
    features: [
      [true, "Shared seats"],
      [true, "Approval flow"],
      [true, "Quota controls"],
      [true, "Dedicated support"],
    ] as const,
  },
];

export default function PricingCards() {
  return (
    <div className="space-y-8">
      <div className="flex items-end justify-between gap-4">
        <h2 className="text-4xl font-bold tracking-[-0.05em] text-white">Pricing</h2>
        <p className="hidden text-sm text-zinc-500 sm:block">Pick a runway and launch.</p>
      </div>
      <div className="grid gap-4 lg:grid-cols-3">
        {plans.map((plan, index) => (
          <motion.div
            key={plan.name}
            initial={{ opacity: 0, y: 26 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, amount: 0.25 }}
            transition={{ duration: 0.45, delay: index * 0.08 }}
          >
            <GlowCard
              glowClassName={plan.glow}
              className={`h-full ${plan.highlight ? "shadow-[0_0_80px_-36px_rgba(34,211,238,0.85)]" : ""}`}
            >
              <div className="flex h-full flex-col gap-8 p-6">
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="text-sm font-semibold uppercase tracking-[0.28em] text-zinc-400">
                      {plan.name}
                    </div>
                    {plan.highlight ? (
                      <span className="rounded-full border border-cyan-300/30 bg-cyan-400/10 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-100">
                        Popular
                      </span>
                    ) : null}
                  </div>
                  <div className="flex items-end gap-2 text-white">
                    <span className="text-5xl font-bold tracking-[-0.05em]">{plan.price}</span>
                    <span className="pb-1 text-sm text-zinc-500">{plan.unit}</span>
                  </div>
                </div>

                <div className="space-y-3 text-sm text-zinc-300">
                  {plan.features.map(([enabled, label]) => (
                    <div key={label} className="flex items-center gap-3 rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-3">
                      <span
                        className={`inline-flex h-8 w-8 items-center justify-center rounded-full border ${
                          enabled
                            ? "border-emerald-300/30 bg-emerald-400/10 text-emerald-200"
                            : "border-zinc-600/40 bg-zinc-800/60 text-zinc-500"
                        }`}
                      >
                        {enabled ? <Check className="h-4 w-4" /> : <X className="h-4 w-4" />}
                      </span>
                      <span>{label}</span>
                    </div>
                  ))}
                </div>

                <button
                  type="button"
                  className={`mt-auto rounded-[1.35rem] border px-4 py-3 text-sm font-semibold transition ${
                    plan.highlight
                      ? "border-cyan-300/30 bg-cyan-400/14 text-cyan-100 hover:bg-cyan-300/18"
                      : "border-white/10 bg-white/[0.04] text-zinc-100 hover:border-white/20 hover:bg-white/[0.07]"
                  }`}
                >
                  Choose {plan.name}
                </button>
              </div>
            </GlowCard>
          </motion.div>
        ))}
      </div>
    </div>
  );
}
