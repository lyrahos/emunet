import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Shield, ArrowRight, UserPlus, X } from "lucide-react";
import Button from "@/components/ui/Button";
import { useIpc } from "@/hooks/useIpc";

interface Guardian {
  id: string;
  name: string;
}

export default function Recovery() {
  const navigate = useNavigate();
  const { call, loading } = useIpc();
  const [guardians, setGuardians] = useState<Guardian[]>([]);
  const [inviteCode, setInviteCode] = useState("");

  const handleAddGuardian = async () => {
    if (!inviteCode.trim()) return;
    const result = await call("add_recovery_guardian", {
      invite_code: inviteCode.trim(),
    });
    if (result) {
      const g = result as { guardian_id: string; display_name: string };
      setGuardians((prev) => [
        ...prev,
        { id: g.guardian_id, name: g.display_name },
      ]);
      setInviteCode("");
    }
  };

  const removeGuardian = (id: string) => {
    setGuardians((prev) => prev.filter((g) => g.id !== id));
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--color-bg)] p-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        className="w-full max-w-md space-y-8"
      >
        <div className="text-center space-y-3">
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            transition={{ type: "spring", stiffness: 200, damping: 15, delay: 0.2 }}
            className="mx-auto w-16 h-16 rounded-2xl bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center"
          >
            <Shield className="h-8 w-8 text-purple-600 dark:text-purple-400" />
          </motion.div>
          <h1 className="text-2xl font-bold text-[var(--color-text)]">
            Recovery Contacts
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)]">
            Add trusted contacts who can help you recover your account if you
            lose access. You need at least 2 of 3 contacts for recovery.
          </p>
        </div>

        {/* Guardian list */}
        <div className="space-y-3">
          {guardians.map((g) => (
            <motion.div
              key={g.id}
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
            >
              <div className="w-9 h-9 rounded-full bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center">
                <span className="text-sm font-semibold text-purple-600 dark:text-purple-400">
                  {g.name.charAt(0).toUpperCase()}
                </span>
              </div>
              <span className="flex-1 text-sm font-medium text-[var(--color-text)]">
                {g.name}
              </span>
              <button
                onClick={() => removeGuardian(g.id)}
                className="p-1 rounded-md text-[var(--color-text-secondary)] hover:text-red-500 transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            </motion.div>
          ))}

          {/* Add guardian input */}
          <div className="flex gap-2">
            <input
              type="text"
              value={inviteCode}
              onChange={(e) => setInviteCode(e.target.value)}
              placeholder="Paste contact invite code"
              className="flex-1 px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
            />
            <Button
              variant="secondary"
              onClick={handleAddGuardian}
              loading={loading}
              disabled={!inviteCode.trim()}
            >
              <UserPlus className="h-4 w-4" />
            </Button>
          </div>

          <p className="text-xs text-[var(--color-text-secondary)] text-center">
            {guardians.length}/3 recovery contacts added
          </p>
        </div>

        <div className="space-y-3">
          <Button
            className="w-full"
            size="lg"
            onClick={() => navigate("/setup/ready")}
          >
            Continue
            <ArrowRight className="h-4 w-4" />
          </Button>
          <Button
            className="w-full"
            variant="ghost"
            onClick={() => navigate("/setup/ready")}
          >
            Skip for now
          </Button>
        </div>

        {/* Step indicator */}
        <div className="flex justify-center gap-2">
          {[0, 1, 2, 3, 4].map((step) => (
            <div
              key={step}
              className={`w-2 h-2 rounded-full ${
                step === 3 ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
              }`}
            />
          ))}
        </div>
      </motion.div>
    </div>
  );
}
