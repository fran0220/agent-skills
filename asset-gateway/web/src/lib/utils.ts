export function cn(...classes: Array<string | false | null | undefined>) {
  return classes.filter(Boolean).join(" ");
}

export function formatQuota(quota: number, used: number) {
  const remaining = Math.max(quota - used, 0);
  const percentage = quota === 0 ? 0 : Math.min((used / quota) * 100, 100);
  return { remaining, percentage };
}

export function studioPath(kind: string) {
  switch (kind) {
    case "image":
      return "/studio/image";
    case "video":
      return "/studio/video";
    case "audio":
    case "music":
    case "tts":
    case "text":
      return "/studio/audio";
    case "model3d":
      return "/studio/3d";
    case "sprite":
      return "/studio/sprite";
    case "world":
      return "/studio/world";
    default:
      return "/dashboard";
  }
}
