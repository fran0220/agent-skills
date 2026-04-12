import HeroSection from "@/components/landing/hero-section";
import ShowcaseGrid from "@/components/landing/showcase-grid";
import PricingCards from "@/components/pricing-cards";

export default function LandingPage() {
  return (
    <main className="min-h-screen">
      <nav className="fixed top-0 z-50 w-full border-b border-white/8 bg-zinc-950/60 backdrop-blur-xl">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-6 py-4">
          <div className="text-lg font-bold tracking-tight text-white">AssetForge</div>
          <div className="flex items-center gap-4">
            <a href="/dashboard" className="text-sm text-zinc-400 hover:text-white transition">Dashboard</a>
            <a href="/login" className="rounded-full bg-white/10 px-4 py-2 text-sm text-white hover:bg-white/16 transition">Sign In</a>
          </div>
        </div>
      </nav>

      <div className="mx-auto max-w-7xl space-y-24 px-6 pt-28 pb-20">
        <HeroSection />
        <ShowcaseGrid />
        <PricingCards />
      </div>

      <footer className="border-t border-white/8 py-8 text-center text-xs text-zinc-600">
        AssetForge · API · Docs · GitHub
      </footer>
    </main>
  );
}
