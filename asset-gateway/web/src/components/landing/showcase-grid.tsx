"use client";

import Image from "next/image";
import { motion } from "framer-motion";

const items = [
  {
    src: "/showcase/showcase-crystal-dragon.png",
    alt: "Crystal dragon",
    width: 1792,
    height: 1024,
    category: "Image",
    label: "Crystal Dragon",
    span: "md:col-span-2",
  },
  {
    src: "/showcase/showcase-ramen-shop.png",
    alt: "Isometric ramen shop",
    width: 1024,
    height: 1024,
    category: "Image",
    label: "Ramen Shop",
    span: "",
  },
  {
    src: "/showcase/showcase-cyber-samurai.png",
    alt: "Cyber samurai",
    width: 1024,
    height: 1024,
    category: "Image",
    label: "Cyber Samurai",
    span: "",
  },
  {
    src: "/showcase/showcase-brutalist-museum.png",
    alt: "Brutalist museum interior",
    width: 1792,
    height: 1024,
    category: "Image",
    label: "Brutalist Museum",
    span: "md:col-span-2",
  },
  {
    src: "/showcase/showcase-holo-phone.png",
    alt: "Holographic phone UI",
    width: 1024,
    height: 1024,
    category: "Image",
    label: "Holo Phone",
    span: "",
  },
  {
    src: "/showcase/showcase-knight-walk.png",
    alt: "Knight walk cycle sprite sheet",
    width: 1792,
    height: 1024,
    category: "Sprite",
    label: "Walk Cycle",
    span: "md:col-span-2",
  },
];

export default function ShowcaseGrid() {
  return (
    <section className="space-y-8">
      <h2 className="text-4xl font-bold tracking-[-0.05em] text-white">
        Made with AssetForge
      </h2>

      <div className="grid gap-4 md:grid-cols-3">
        {items.map((item, i) => (
          <motion.div
            key={item.src}
            initial={{ opacity: 0, y: 28 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, amount: 0.15 }}
            transition={{ duration: 0.45, delay: i * 0.07 }}
            className={`group relative overflow-hidden rounded-[1.5rem] border border-white/10 ${item.span}`}
          >
            <div className="relative aspect-[16/10] overflow-hidden">
              <Image
                src={item.src}
                alt={item.alt}
                fill
                unoptimized
                className="object-cover transition duration-500 group-hover:scale-105 group-hover:brightness-110"
              />
            </div>

            <div className="pointer-events-none absolute inset-x-0 bottom-0 flex items-end justify-between p-4">
              <span className="rounded-full border border-white/12 bg-zinc-950/60 px-2.5 py-1 text-[11px] font-medium text-zinc-200 backdrop-blur-xl">
                {item.category}
              </span>
              <span className="rounded-full border border-white/12 bg-zinc-950/60 px-2.5 py-1 text-[11px] font-medium text-zinc-300 backdrop-blur-xl">
                {item.label}
              </span>
            </div>
          </motion.div>
        ))}
      </div>
    </section>
  );
}
