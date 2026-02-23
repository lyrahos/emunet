import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  Shield,
  UserPlus,
  X,
  CheckCircle2,
  AlertTriangle,
  Heart,
} from "lucide-react";
import Button from "@/components/ui/Button";
import Card from "@/components/ui/Card";
import { rpcCall } from "@/lib/rpc-client";
import { useIpc } from "@/hooks/useIpc";
import { useAppStore } from "@/lib/store";

interface Guardian {
  id: string;
  displayName: string;
  status: "active" | "pending" | "offline";
  addedAt: number;
}

interface RecoveryHealth {
  guardianCount: number;
  threshold: number;
  healthy: boolean;
}

export default function RecoverySetup() {
  const { call, loading } = useIpc();
  const addToast = useAppStore((s) => s.addToast);

  const [guardians, setGuardians] = useState<Guardian[]>([]);
  const [health, setHealth] = useState<RecoveryHealth>({
    guardianCount: 0,
    threshold: 2,
    healthy: false,
  });
  const [inviteCode, setInviteCode] = useState("");

  useEffect(() => {
    rpcCall<Guardian[]>("get_recovery_guardians")
      .then(setGuardians)
      .catch(() => {});
    rpcCall<RecoveryHealth>("get_recovery_health")
      .then(setHealth)
      .catch(() => {});
  }, []);

  const handleAddGuardian = async () => {
    if (!inviteCode.trim()) return;
    const result = await call("add_recovery_guardian", {
      invite_code: inviteCode.trim(),
    });
    if (result) {
      const g = result as { guardian_id: string; display_name: string };
      setGuardians((prev) => [
        ...prev,
        {
          id: g.guardian_id,
          displayName: g.display_name,
          status: "pending",
          addedAt: Date.now(),
        },
      ]);
      setInviteCode("");
      setHealth((prev) => ({
        ...prev,
        guardianCount: prev.guardianCount + 1,
        healthy: prev.guardianCount + 1 >= prev.threshold,
      }));
      addToast({ type: "success", message: `Recovery contact added.` });
    }
  };

  const handleRemoveGuardian = async (id: string) => {
    try {
      await rpcCall("remove_recovery_guardian", { guardian_id: id });
      setGuardians((prev) => prev.filter((g) => g.id !== id));
      setHealth((prev) => ({
        ...prev,
        guardianCount: prev.guardianCount - 1,
        healthy: prev.guardianCount - 1 >= prev.threshold,
      }));
      addToast({ type: "info", message: "Recovery contact removed." });
    } catch {
      addToast({ type: "error", message: "Failed to remove contact." });
    }
  };

  const statusColors = {
    active: "text-green-500",
    pending: "text-yellow-500",
    offline: "text-gray-400",
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">
        <div>
          <h1 className="text-xl font-bold text-[var(--color-text)]">
            Account Recovery
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)] mt-1">
            Manage your recovery contacts. You need at least{" "}
            {health.threshold} of 3 to recover your account.
          </p>
        </div>

        {/* Health card */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <Card
            padding="lg"
            className={
              health.healthy
                ? "!border-green-200 dark:!border-green-900/50"
                : "!border-yellow-200 dark:!border-yellow-900/50"
            }
          >
            <div className="flex items-center gap-4">
              <div
                className={`w-12 h-12 rounded-xl flex items-center justify-center ${
                  health.healthy
                    ? "bg-green-100 dark:bg-green-900/30"
                    : "bg-yellow-100 dark:bg-yellow-900/30"
                }`}
              >
                {health.healthy ? (
                  <CheckCircle2 className="h-6 w-6 text-green-500" />
                ) : (
                  <AlertTriangle className="h-6 w-6 text-yellow-500" />
                )}
              </div>
              <div>
                <h2 className="text-base font-semibold text-[var(--color-text)]">
                  {health.healthy ? "Recovery Ready" : "Recovery Not Ready"}
                </h2>
                <p className="text-sm text-[var(--color-text-secondary)]">
                  {health.guardianCount} of {health.threshold} required contacts
                  added
                </p>
              </div>
            </div>

            {/* Progress bar */}
            <div className="mt-4 w-full h-2 rounded-full bg-[var(--color-border)]">
              <motion.div
                className={`h-full rounded-full ${
                  health.healthy ? "bg-green-500" : "bg-yellow-500"
                }`}
                animate={{
                  width: `${Math.min((health.guardianCount / 3) * 100, 100)}%`,
                }}
                transition={{ type: "spring", stiffness: 200, damping: 25 }}
              />
            </div>

            {/* Dots */}
            <div className="flex justify-between mt-2">
              {[0, 1, 2].map((i) => (
                <div
                  key={i}
                  className={`flex items-center gap-1 text-xs ${
                    i < health.guardianCount
                      ? "text-green-500"
                      : "text-[var(--color-text-secondary)]"
                  }`}
                >
                  <Heart
                    className={`h-3 w-3 ${
                      i < health.guardianCount ? "fill-green-500" : ""
                    }`}
                  />
                  Contact {i + 1}
                </div>
              ))}
            </div>
          </Card>
        </motion.div>

        {/* Guardian list */}
        <div className="space-y-3">
          <h2 className="text-base font-semibold text-[var(--color-text)]">
            Recovery Contacts
          </h2>

          {guardians.map((g, i) => (
            <motion.div
              key={g.id}
              initial={{ opacity: 0, y: 5 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              className="flex items-center gap-3 p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
            >
              <div className="w-10 h-10 rounded-full bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center">
                <Shield className="h-5 w-5 text-purple-500" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-[var(--color-text)] truncate">
                  {g.displayName}
                </p>
                <p
                  className={`text-xs capitalize ${statusColors[g.status]}`}
                >
                  {g.status}
                </p>
              </div>
              <button
                onClick={() => handleRemoveGuardian(g.id)}
                className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-red-500 transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            </motion.div>
          ))}

          {guardians.length < 3 && (
            <div className="flex gap-2">
              <input
                type="text"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleAddGuardian()}
                placeholder="Paste contact invite code"
                className="flex-1 px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
              />
              <Button
                onClick={handleAddGuardian}
                loading={loading}
                disabled={!inviteCode.trim()}
              >
                <UserPlus className="h-4 w-4" />
                Add
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
