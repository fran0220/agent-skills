import type { AssetKind } from "@/lib/categories";

type AssetCard = {
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

type ActiveJob = {
  id: string;
  kind: AssetKind;
  label: string;
  progress: number;
  eta: string;
  status: "rendering" | "processing" | "complete";
  preview?: string;
};

export const quota = {
  total: 500,
  used: 312,
};

export const dashboardAssets: AssetCard[] = [
  {
    id: "ast_img_01",
    kind: "image",
    title: "Floating Islands",
    prompt: "ethereal floating islands above misty ocean, volumetric golden hour light",
    createdAt: "4m ago",
    preview: "/showcase/hero-floating-islands.png",
    height: 320,
  },
  {
    id: "ast_img_02",
    kind: "image",
    title: "Crystal Dragon",
    prompt: "ancient crystal dragon emerging from volcanic cave, bioluminescent scales",
    createdAt: "9m ago",
    preview: "/showcase/showcase-crystal-dragon.png",
    height: 300,
  },
  {
    id: "ast_spr_01",
    kind: "sprite",
    title: "Knight Walk",
    prompt: "armored knight character, medieval fantasy style, walk cycle",
    createdAt: "18m ago",
    preview: "/showcase/showcase-knight-walk.png",
    height: 240,
  },
  {
    id: "ast_mus_01",
    kind: "music",
    title: "Epic Theme",
    prompt: "uplifting cinematic orchestral theme, game title screen",
    createdAt: "27m ago",
    preview: "/showcase/showcase-ramen-shop.png",
    height: 240,
  },
  {
    id: "ast_img_03",
    kind: "image",
    title: "Ramen Shop",
    prompt: "isometric cozy Japanese ramen shop at night, rain outside",
    createdAt: "35m ago",
    preview: "/showcase/showcase-ramen-shop.png",
    height: 340,
  },
  {
    id: "ast_img_04",
    kind: "image",
    title: "Cyber Samurai",
    prompt: "futuristic cyber samurai warrior, neon armor with glowing cyan accents",
    createdAt: "41m ago",
    preview: "/showcase/showcase-cyber-samurai.png",
    height: 380,
  },
  {
    id: "ast_img_05",
    kind: "image",
    title: "Holo Phone",
    prompt: "glass smartphone floating in dark void, holographic UI projecting",
    createdAt: "53m ago",
    preview: "/showcase/showcase-holo-phone.png",
    height: 340,
  },
  {
    id: "ast_img_06",
    kind: "image",
    title: "Museum Interior",
    prompt: "brutalist concrete museum interior with massive skylights, tropical garden",
    createdAt: "1h ago",
    preview: "/showcase/showcase-brutalist-museum.png",
    height: 300,
  },
];

export const publicGallery: AssetCard[] = dashboardAssets.map((item, index) => ({
  ...item,
  id: `${item.id}_public`,
  author: ["Mina", "Kai", "Rin", "Jules", "Sora", "Nyx", "Ava", "Zen"][index % 8],
  likes: 42 + index * 9,
}));

export const activeJobs: ActiveJob[] = [
  {
    id: "job_01",
    kind: "image",
    label: "Poster variant",
    progress: 72,
    eta: "22s",
    status: "rendering",
  },
  {
    id: "job_02",
    kind: "video",
    label: "Camera pass",
    progress: 46,
    eta: "1m 10s",
    status: "processing",
  },
  {
    id: "job_03",
    kind: "sprite",
    label: "Walk loop",
    progress: 100,
    eta: "ready",
    status: "complete",
    preview: "/showcase/showcase-knight-walk.png",
  },
];

export const apiKeys = [
  { prefix: "agk_live_91a2", createdAt: "2026-04-07", lastUsed: "3m ago" },
  { prefix: "agk_team_7bd4", createdAt: "2026-04-01", lastUsed: "1h ago" },
  { prefix: "agk_ops_14f1", createdAt: "2026-03-28", lastUsed: "2d ago" },
];

export const usageBars = [48, 66, 39, 81, 58, 92, 72, 64, 86, 59, 70, 88];

export const sessionFrames = Array.from({ length: 6 }, (_, index) => ({
  id: `frame_${index + 1}`,
  label: `v${index + 1}`,
  preview: "/showcase/hero-floating-islands.png",
}));

export const spriteFrames = Array.from({ length: 8 }, (_, index) => ({
  id: `spr_frame_${index + 1}`,
  preview: "/showcase/showcase-knight-walk.png",
}));

export const audioHistory = Array.from({ length: 6 }, (_, index) => ({
  id: `aud_hist_${index + 1}`,
  title: ["Impact", "Rush", "Bloom", "Subdrop", "Arc", "Signal"][index],
  kind: index % 2 === 0 ? "audio" : "music",
}));

export const pipelineSteps = ["Generate", "Texture", "Rig", "Animate", "Export"];
