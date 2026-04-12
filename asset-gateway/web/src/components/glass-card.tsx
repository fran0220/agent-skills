import type { ComponentPropsWithoutRef } from "react";
import { cn } from "@/lib/utils";

type GlassCardProps = ComponentPropsWithoutRef<"div">;

export function GlassCard({ className, ...props }: GlassCardProps) {
  return (
    <div
      className={cn(
        "rounded-[28px] border border-white/10 bg-white/[0.045] backdrop-blur-xl shadow-[0_24px_80px_rgba(0,0,0,0.32)]",
        className,
      )}
      {...props}
    />
  );
}
