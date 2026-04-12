"use client";

import { AnimatePresence, motion } from "framer-motion";
import { ChevronDown, Download } from "lucide-react";
import Image from "next/image";
import { useState } from "react";
import { CATEGORY_META } from "@/lib/categories";
import { activeJobs } from "@/lib/mock-data";
import { cn } from "@/lib/utils";

export function JobTracker() {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <motion.aside
      initial={{ opacity: 0, y: 24 }}
      animate={{ opacity: 1, y: 0 }}
      className="fixed bottom-6 right-6 z-50 w-[340px] max-w-[calc(100vw-2rem)] rounded-[28px] border border-white/10 bg-black/60 p-4 backdrop-blur-2xl shadow-[0_28px_90px_rgba(0,0,0,0.42)]"
    >
      <button
        onClick={() => setCollapsed((value) => !value)}
        className="flex w-full items-center justify-between rounded-2xl px-2 py-1 text-left"
      >
        <div>
          <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">Job tracker</p>
          <p className="mt-1 text-sm text-white">{activeJobs.length} active renders</p>
        </div>
        <ChevronDown
          className={cn("h-4 w-4 text-zinc-400 transition", collapsed && "rotate-180")}
        />
      </button>

      <AnimatePresence initial={false}>
        {!collapsed && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="overflow-hidden"
          >
            <div className="mt-4 space-y-3">
              {activeJobs.map((job) => {
                const meta = CATEGORY_META[job.kind];
                const Icon = meta.icon;
                const circumference = 2 * Math.PI * 22;
                const dashOffset = circumference - (job.progress / 100) * circumference;

                return (
                  <div
                    key={job.id}
                    className="flex items-center gap-3 rounded-[24px] border border-white/8 bg-white/[0.04] p-3"
                  >
                    <div className="relative flex h-14 w-14 items-center justify-center">
                      <svg className="absolute inset-0 -rotate-90" viewBox="0 0 52 52">
                        <circle cx="26" cy="26" r="22" stroke="rgba(255,255,255,0.08)" strokeWidth="4" fill="none" />
                        <circle
                          cx="26"
                          cy="26"
                          r="22"
                          stroke="currentColor"
                          strokeWidth="4"
                          fill="none"
                          strokeDasharray={circumference}
                          strokeDashoffset={dashOffset}
                          className={meta.color}
                        />
                      </svg>
                      {job.status === "complete" && job.preview ? (
                        <Image
                          src={job.preview}
                          alt={job.label}
                          width={44}
                          height={44}
                          unoptimized
                          className="h-11 w-11 rounded-2xl object-cover"
                        />
                      ) : (
                        <Icon className={cn("h-5 w-5", meta.color)} />
                      )}
                    </div>

                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <p className="truncate text-sm font-medium text-white">{job.label}</p>
                        <span className="text-xs text-zinc-500">{job.eta}</span>
                      </div>
                      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white/8">
                        <motion.div
                          initial={{ width: 0 }}
                          animate={{ width: `${job.progress}%` }}
                          className="h-full rounded-full bg-white"
                        />
                      </div>
                    </div>

                    {job.status === "complete" ? (
                      <button className="flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-white/8 text-white hover:bg-white/12">
                        <Download className="h-4 w-4" />
                      </button>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.aside>
  );
}
