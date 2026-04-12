"use client";

import { motion } from "framer-motion";
import { useMemo, useState } from "react";
import { AssetCard } from "@/components/asset-card";
import { GlassCard } from "@/components/glass-card";
import { CATEGORY_META, DASHBOARD_KINDS, type AssetKind } from "@/lib/categories";
import { publicGallery } from "@/lib/mock-data";
import { cn } from "@/lib/utils";

export default function GalleryPage() {
  const [selectedKinds, setSelectedKinds] = useState<AssetKind[]>([]);

  const items = useMemo(() => {
    if (selectedKinds.length === 0) {
      return publicGallery;
    }
    return publicGallery.filter((item) => selectedKinds.includes(item.kind));
  }, [selectedKinds]);

  return (
    <div className="space-y-5 pb-24">
      <GlassCard className="p-6">
        <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <p className="text-xs uppercase tracking-[0.32em] text-zinc-500">Public gallery</p>
            <h1 className="mt-3 text-5xl leading-none">Output worth stealing glances at.</h1>
          </div>
          <div className="flex flex-wrap gap-2">
            {DASHBOARD_KINDS.map((kind) => {
              const meta = CATEGORY_META[kind];
              const Icon = meta.icon;
              const active = selectedKinds.includes(kind);
              return (
                <motion.button
                  key={kind}
                  whileTap={{ scale: 0.97 }}
                  whileHover={{ y: -2 }}
                  onClick={() =>
                    setSelectedKinds((current) =>
                      current.includes(kind)
                        ? current.filter((item) => item !== kind)
                        : [...current, kind],
                    )
                  }
                  title={meta.label}
                  className={cn(
                    "flex h-14 w-14 items-center justify-center rounded-2xl border transition",
                    active
                      ? "border-white/18 bg-white/12 text-white"
                      : "border-white/8 bg-white/[0.04] text-zinc-500 hover:text-white",
                  )}
                >
                  <Icon className={cn("h-5 w-5", meta.color)} />
                </motion.button>
              );
            })}
          </div>
        </div>
      </GlassCard>

      <div className="columns-1 gap-5 md:columns-2 xl:columns-3 2xl:columns-4">
        {items.map((asset) => (
          <AssetCard key={asset.id} asset={asset} publicMode />
        ))}
      </div>
    </div>
  );
}
