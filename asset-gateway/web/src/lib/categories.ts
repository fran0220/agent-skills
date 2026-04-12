import type { LucideIcon } from "lucide-react";
import {
  AudioLines,
  Box,
  Clapperboard,
  Globe2,
  ImageIcon,
  Mic2,
  Music4,
  Sparkles,
  Type,
} from "lucide-react";

export type AssetKind =
  | "image"
  | "video"
  | "audio"
  | "music"
  | "tts"
  | "model3d"
  | "sprite"
  | "world"
  | "text";

export type CategoryMeta = {
  label: string;
  icon: LucideIcon;
  color: string;
  glow: string;
  ring: string;
  soft: string;
};

export const CATEGORY_META: Record<AssetKind, CategoryMeta> = {
  image: {
    label: "Image",
    icon: ImageIcon,
    color: "text-cyan-300",
    glow: "shadow-[0_0_40px_rgba(34,211,238,0.24)]",
    ring: "ring-cyan-400/40",
    soft: "from-cyan-400/20 to-cyan-500/5",
  },
  video: {
    label: "Video",
    icon: Clapperboard,
    color: "text-violet-300",
    glow: "shadow-[0_0_40px_rgba(167,139,250,0.22)]",
    ring: "ring-violet-400/40",
    soft: "from-violet-400/20 to-violet-500/5",
  },
  audio: {
    label: "Audio",
    icon: AudioLines,
    color: "text-amber-300",
    glow: "shadow-[0_0_40px_rgba(251,191,36,0.22)]",
    ring: "ring-amber-400/40",
    soft: "from-amber-400/20 to-amber-500/5",
  },
  music: {
    label: "Music",
    icon: Music4,
    color: "text-pink-300",
    glow: "shadow-[0_0_40px_rgba(244,114,182,0.22)]",
    ring: "ring-pink-400/40",
    soft: "from-pink-400/20 to-pink-500/5",
  },
  tts: {
    label: "TTS",
    icon: Mic2,
    color: "text-emerald-300",
    glow: "shadow-[0_0_40px_rgba(52,211,153,0.22)]",
    ring: "ring-emerald-400/40",
    soft: "from-emerald-400/20 to-emerald-500/5",
  },
  model3d: {
    label: "3D",
    icon: Box,
    color: "text-blue-300",
    glow: "shadow-[0_0_40px_rgba(96,165,250,0.22)]",
    ring: "ring-blue-400/40",
    soft: "from-blue-400/20 to-blue-500/5",
  },
  sprite: {
    label: "Sprite",
    icon: Sparkles,
    color: "text-orange-300",
    glow: "shadow-[0_0_40px_rgba(251,146,60,0.22)]",
    ring: "ring-orange-400/40",
    soft: "from-orange-400/20 to-orange-500/5",
  },
  world: {
    label: "World",
    icon: Globe2,
    color: "text-teal-300",
    glow: "shadow-[0_0_40px_rgba(45,212,191,0.22)]",
    ring: "ring-teal-400/40",
    soft: "from-teal-400/20 to-teal-500/5",
  },
  text: {
    label: "Text",
    icon: Type,
    color: "text-zinc-200",
    glow: "shadow-[0_0_40px_rgba(212,212,216,0.18)]",
    ring: "ring-zinc-400/30",
    soft: "from-zinc-300/10 to-zinc-500/5",
  },
};

export const DASHBOARD_KINDS: AssetKind[] = [
  "image",
  "video",
  "audio",
  "music",
  "tts",
  "model3d",
  "sprite",
  "world",
  "text",
];

export const STUDIO_KINDS: AssetKind[] = [
  "image",
  "video",
  "audio",
  "music",
  "model3d",
  "sprite",
  "world",
];
