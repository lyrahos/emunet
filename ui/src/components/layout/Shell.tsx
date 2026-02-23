import { Outlet, Navigate } from "react-router-dom";
import { useAppStore } from "@/lib/store";
import Sidebar from "./Sidebar";
import { Lock } from "lucide-react";
import Button from "@/components/ui/Button";
import { useState } from "react";
import { rpcCall } from "@/lib/rpc-client";

function LockScreen() {
  const [password, setPassword] = useState("");
  const [unlocking, setUnlocking] = useState(false);
  const [error, setError] = useState("");
  const setAuth = useAppStore((s) => s.setAuth);

  const handleUnlock = async () => {
    setUnlocking(true);
    setError("");
    try {
      const res = await rpcCall<{ pik_hash: string; display_name: string }>(
        "unlock_session",
        { password },
      );
      setAuth({
        unlocked: true,
        pikHash: res.pik_hash,
        displayName: res.display_name,
      });
    } catch {
      setError("Incorrect password. Please try again.");
    } finally {
      setUnlocking(false);
    }
  };

  return (
    <div className="flex items-center justify-center h-screen bg-[var(--color-bg)]">
      <div className="w-full max-w-sm p-8 space-y-6 text-center">
        <div className="mx-auto w-16 h-16 rounded-2xl bg-[var(--color-accent)]/10 flex items-center justify-center">
          <Lock className="h-8 w-8 text-[var(--color-accent)]" />
        </div>
        <div>
          <h1 className="text-xl font-semibold text-[var(--color-text)]">
            Session Locked
          </h1>
          <p className="mt-1 text-sm text-[var(--color-text-secondary)]">
            Enter your password to unlock Ochra.
          </p>
        </div>
        <div className="space-y-3">
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleUnlock()}
            placeholder="Password"
            className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50"
          />
          {error && (
            <p className="text-sm text-red-500">{error}</p>
          )}
          <Button
            className="w-full"
            loading={unlocking}
            onClick={handleUnlock}
          >
            Unlock
          </Button>
        </div>
      </div>
    </div>
  );
}

export default function Shell() {
  const auth = useAppStore((s) => s.auth);

  // If setup hasn't been completed, redirect to setup wizard
  if (!auth.setupComplete) {
    return <Navigate to="/setup" replace />;
  }

  // If session is locked, show lock screen
  if (!auth.unlocked) {
    return <LockScreen />;
  }

  return (
    <div className="flex h-screen bg-[var(--color-bg)]">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  );
}
