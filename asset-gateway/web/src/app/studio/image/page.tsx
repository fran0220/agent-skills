import { ImageStudioView } from "@/components/image-studio-view";

const DEFAULT_PROMPT = "Glass pavilion in cyan rain, ultra crisp moodboard lighting";

type ImageStudioPageProps = {
  searchParams?: Promise<{
    prompt?: string | string[];
  }>;
};

export default async function ImageStudioPage({ searchParams }: ImageStudioPageProps) {
  const params = searchParams ? await searchParams : undefined;
  const promptValue = params?.prompt;
  const prompt =
    typeof promptValue === "string"
      ? promptValue
      : Array.isArray(promptValue)
        ? promptValue[0] ?? DEFAULT_PROMPT
        : DEFAULT_PROMPT;

  return <ImageStudioView prompt={prompt} />;
}
