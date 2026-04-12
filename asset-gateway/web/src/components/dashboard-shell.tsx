"use client";

import { motion } from "framer-motion";
import {
  GalleryVerticalEnd,
  KeyRound,
  LayoutDashboard,
  Search,
  Settings,
} from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import { JobTracker } from "@/components/job-tracker";
import { cn } from "@/lib/utils";

const navItems = [
  { href: "/dashboard", icon: LayoutDashboard, label: "Dashboard" },
  { href: "/dashboard/gallery", icon: GalleryVerticalEnd, label: "Gallery" },
  { href: "/dashboard/settings#keys", icon: KeyRound, label: "API Keys" },
  { href: "/dashboard/settings", icon: Settings, label: "Settings" },
];

export function DashboardShell({ children }: { children: ReactNode }) {
  const pathname = usePathname();

  return (
    <div className="min-h-screen bg-zinc-950 text-white">
      <div className="mx-auto flex min-h-screen max-w-[1680px] gap-5 px-4 py-4 lg:px-6">
        <aside className="hidden w-[92px] shrink-0 rounded-[32px] border border-white/10 bg-black/45 p-4 backdrop-blur-2xl lg:flex lg:flex-col lg:items-center lg:justify-between">
          <div className="flex flex-col items-center gap-4">
            <div className="grid h-14 w-14 place-items-center rounded-[22px] border border-cyan-400/20 bg-cyan-400/10 text-cyan-300 shadow-[0_0_40px_rgba(34,211,238,0.18)]">
              AG
            </div>
            {navItems.map((item, index) => {
              const Icon = item.icon;
              const active = item.href === "/dashboard/settings#keys"
                ? pathname.startsWith("/dashboard/settings")
                : pathname === item.href;
              return (
                <motion.div
                  key={item.href}
                  initial={{ opacity: 0, x: -12 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.05 }}
                >
                  <Link
                    href={item.href}
                    title={item.label}
                    className={cn(
                      "group relative flex h-14 w-14 items-center justify-center rounded-[22px] border transition",
                      active
                        ? "border-white/20 bg-white/12 text-white"
                        : "border-white/8 bg-white/[0.04] text-zinc-500 hover:text-white",
                    )}
                  >
                    <Icon className="h-5 w-5" />
                    <span className="pointer-events-none absolute left-[74px] rounded-full bg-black/80 px-3 py-1 text-[10px] uppercase tracking-[0.2em] text-white opacity-0 transition group-hover:opacity-100">
                      {item.label}
                    </span>
                  </Link>
                </motion.div>
              );
            })}
          </div>

          <div className="flex h-14 w-14 items-center justify-center rounded-[22px] border border-white/10 bg-white/[0.04] text-sm font-medium text-white/70">
            F
          </div>
        </aside>

        <div className="min-w-0 flex-1">
          <header className="mb-5 flex flex-col gap-4 rounded-[30px] border border-white/10 bg-black/35 px-4 py-4 backdrop-blur-2xl md:flex-row md:items-center md:justify-between md:px-6">
            <div className="flex max-w-xl flex-1 items-center gap-3 rounded-[22px] border border-white/10 bg-white/[0.04] px-4 py-3">
              <Search className="h-4 w-4 text-zinc-500" />
              <input
                placeholder="Search prompts, outputs, sessions"
                className="w-full bg-transparent text-sm text-white outline-none placeholder:text-zinc-500"
              />
            </div>
            <div className="flex items-center gap-4">
              <div className="text-right">
                <p className="text-xs uppercase tracking-[0.28em] text-zinc-500">workspace</p>
                <p className="text-sm font-medium text-white">upload.xiaomao.chat</p>
              </div>
              <div className="grid h-12 w-12 place-items-center rounded-full border border-white/10 bg-[linear-gradient(135deg,rgba(34,211,238,0.3),rgba(236,72,153,0.25))] text-sm font-semibold text-white">
                FM
              </div>
            </div>
          </header>

          <main>{children}</main>
        </div>
      </div>
      <JobTracker />
    </div>
  );
}
