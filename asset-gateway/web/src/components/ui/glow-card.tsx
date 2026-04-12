"use client";

import { motion } from "framer-motion";
import { ReactNode } from "react";

type GlowCardProps = {
  children: ReactNode;
  className?: string;
  glowClassName?: string;
};

export default function GlowCard({
  children,
  className = "",
  glowClassName = "from-cyan-400/25 via-fuchsia-400/10 to-emerald-400/20",
}: GlowCardProps) {
  return (
    <motion.div
      whileHover={{ y: -8, scale: 1.015 }}
      transition={{ type: "spring", stiffness: 240, damping: 20 }}
      className={`group relative overflow-hidden rounded-[2rem] border border-white/10 bg-zinc-950/70 ${className}`}
    >
      <div
        className={`pointer-events-none absolute inset-0 bg-gradient-to-br opacity-0 blur-2xl transition duration-500 group-hover:opacity-100 ${glowClassName}`}
      />
      <div className="pointer-events-none absolute inset-px rounded-[calc(2rem-1px)] border border-white/10 opacity-40 transition duration-500 group-hover:opacity-100" />
      <div className="relative h-full rounded-[calc(2rem-1px)] bg-[linear-gradient(180deg,rgba(255,255,255,0.06),rgba(255,255,255,0.02))]">
        {children}
      </div>
    </motion.div>
  );
}
