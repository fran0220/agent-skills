"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { FormEvent, useMemo, useState } from "react";
import { ArrowRight, GitBranch, KeyRound, LoaderCircle, Shield } from "lucide-react";

const fallbackApiUrl = "https://upload.xiaomao.chat";

export default function LoginPage() {
  const router = useRouter();
  const apiUrl = useMemo(
    () => process.env.NEXT_PUBLIC_ASSET_GATEWAY_URL ?? fallbackApiUrl,
    []
  );
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token.trim()) {
      setError("Enter an API key or admin token.");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${apiUrl}/auth/login`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ token: token.trim() }),
      });

      const payload = (await response.json()) as {
        ok?: boolean;
        error?: { message?: string };
      };

      if (!response.ok || !payload.ok) {
        throw new Error(payload.error?.message ?? "Login failed.");
      }

      window.localStorage.setItem("assetforge_token", token.trim());
      router.push("/");
      router.refresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Login failed.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="mx-auto flex min-h-[calc(100vh-5rem)] w-full max-w-7xl items-center justify-center px-4 py-16 sm:px-6 lg:px-8">
      <div className="grid w-full max-w-5xl gap-8 lg:grid-cols-[0.9fr_1.1fr]">
        <section className="glass-panel flex flex-col justify-between rounded-[2rem] p-8">
          <div className="space-y-5">
            <div className="inline-flex h-12 w-12 items-center justify-center rounded-2xl border border-cyan-400/30 bg-cyan-400/10 text-cyan-200 shadow-[0_0_35px_-15px_rgba(34,211,238,0.85)]">
              <Shield className="h-5 w-5" />
            </div>
            <div className="space-y-3">
              <p className="text-xs font-semibold tracking-[0.32em] text-zinc-500 uppercase">
                Secure Gateway Access
              </p>
              <h1 className="text-4xl font-bold tracking-[-0.04em] text-white">
                Bring your key.
              </h1>
              <p className="max-w-sm text-sm leading-6 text-zinc-400">
                One token unlocks the full asset pipeline.
              </p>
            </div>
          </div>

          <div className="space-y-4 pt-8 text-sm text-zinc-400">
            <div className="rounded-2xl border border-white/8 bg-white/5 p-4">
              <div className="flex items-center justify-between text-zinc-300">
                <span>POST</span>
                <span>/auth/login</span>
              </div>
              <p className="mt-2 text-xs text-zinc-500">{apiUrl}</p>
            </div>
            <Link href="/" className="inline-flex items-center gap-2 text-zinc-300 transition hover:text-white">
              Back to Studio
              <ArrowRight className="h-4 w-4" />
            </Link>
          </div>
        </section>

        <section className="glass-panel rounded-[2rem] p-8 shadow-[0_24px_90px_-45px_rgba(168,85,247,0.85)]">
          <form className="space-y-6" onSubmit={handleSubmit}>
            <div className="space-y-3">
              <label htmlFor="token" className="text-xs font-semibold tracking-[0.28em] text-zinc-500 uppercase">
                API Key
              </label>
              <div className="flex items-center gap-3 rounded-[1.5rem] border border-white/10 bg-zinc-950/70 px-4 py-4">
                <KeyRound className="h-5 w-5 text-cyan-300" />
                <input
                  id="token"
                  type="password"
                  value={token}
                  onChange={(event) => setToken(event.target.value)}
                  placeholder="agk_..."
                  className="w-full bg-transparent text-sm text-zinc-100 outline-none placeholder:text-zinc-600"
                  autoComplete="off"
                />
              </div>
            </div>

            <button
              type="submit"
              disabled={loading}
              className="inline-flex w-full items-center justify-center gap-2 rounded-[1.5rem] border border-cyan-300/30 bg-cyan-400/14 px-5 py-4 text-sm font-semibold text-cyan-100 shadow-[0_0_50px_-22px_rgba(34,211,238,0.95)] transition hover:scale-[1.01] hover:border-cyan-200/50 hover:bg-cyan-300/18 disabled:cursor-not-allowed disabled:opacity-70"
            >
              {loading ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <ArrowRight className="h-4 w-4" />}
              Continue
            </button>

            {error ? (
              <p className="rounded-2xl border border-rose-400/20 bg-rose-400/8 px-4 py-3 text-sm text-rose-200">
                {error}
              </p>
            ) : null}
          </form>

          <div className="mt-8 grid gap-3 sm:grid-cols-2">
            <button
              type="button"
              disabled
              className="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.03] px-4 py-4 text-left text-sm text-zinc-500"
            >
              <span className="mb-3 inline-flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-zinc-900 text-zinc-300">
                <GitBranch className="h-4 w-4" />
              </span>
              <div className="font-medium text-zinc-300">GitHub OAuth</div>
              <div className="mt-1 text-xs uppercase tracking-[0.24em]">Coming Soon</div>
            </button>
            <button
              type="button"
              disabled
              className="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.03] px-4 py-4 text-left text-sm text-zinc-500"
            >
              <span className="mb-3 inline-flex h-10 w-10 items-center justify-center rounded-2xl border border-white/10 bg-zinc-900 text-zinc-300">
                <Shield className="h-4 w-4" />
              </span>
              <div className="font-medium text-zinc-300">SSO</div>
              <div className="mt-1 text-xs uppercase tracking-[0.24em]">Reserved</div>
            </button>
          </div>
        </section>
      </div>
    </main>
  );
}
