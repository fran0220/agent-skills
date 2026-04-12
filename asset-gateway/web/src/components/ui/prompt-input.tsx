"use client";

import { motion } from "framer-motion";
import { Paperclip, SendHorizonal } from "lucide-react";
import { useState } from "react";

const quickTags = [
  "Image",
  "Video",
  "Audio",
  "Music",
  "TTS",
  "Voice",
  "3D Model",
  "Sprite",
  "3D World",
  "Text",
  "Process",
];

export default function PromptInput() {
  const [dragActive, setDragActive] = useState(false);

  return (
    <motion.section
      initial={{ opacity: 0, y: 24 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, amount: 0.35 }}
      transition={{ duration: 0.6, ease: "easeOut" }}
      className="glass-panel relative mx-auto w-full max-w-5xl overflow-hidden rounded-[2rem] p-4 shadow-[0_30px_110px_-50px_rgba(34,211,238,0.65)] sm:p-6"
    >
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(34,211,238,0.18),transparent_34%),radial-gradient(circle_at_bottom_right,rgba(168,85,247,0.16),transparent_28%)]" />
      <div
        onDragEnter={() => setDragActive(true)}
        onDragOver={(event) => {
          event.preventDefault();
          setDragActive(true);
        }}
        onDragLeave={() => setDragActive(false)}
        onDrop={(event) => {
          event.preventDefault();
          setDragActive(false);
        }}
        className={`relative rounded-[1.6rem] border px-4 py-4 transition sm:px-5 sm:py-5 ${
          dragActive
            ? "border-cyan-300/55 bg-cyan-400/10"
            : "border-white/10 bg-zinc-950/65"
        }`}
      >
        <div className="flex items-start gap-4">
          <div className="mt-1 inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl border border-cyan-400/30 bg-cyan-400/10 text-cyan-200">
            <Paperclip className="h-4 w-4" />
          </div>
          <div className="min-w-0 flex-1">
            <textarea
              rows={3}
              placeholder="Describe the asset you want to generate..."
              className="w-full resize-none bg-transparent text-base text-zinc-100 outline-none placeholder:text-zinc-500 sm:text-lg"
            />
            <div className="mt-4 flex flex-wrap gap-2">
              {quickTags.map((tag) => (
                <button
                  key={tag}
                  type="button"
                  className="rounded-full border border-white/8 bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-zinc-300 transition hover:border-cyan-300/40 hover:text-white"
                >
                  {tag}
                </button>
              ))}
            </div>
          </div>
          <button
            type="button"
            className="inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/30 bg-cyan-400/14 text-cyan-100 shadow-[0_0_45px_-20px_rgba(34,211,238,0.95)] transition hover:scale-[1.04] hover:bg-cyan-300/18"
            aria-label="Send prompt"
          >
            <SendHorizonal className="h-4 w-4" />
          </button>
        </div>
      </div>
    </motion.section>
  );
}
