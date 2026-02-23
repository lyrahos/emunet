import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Sprout, HardDrive, Shield, Check } from "lucide-react";
import Button from "@/components/ui/Button";
import { useAppStore } from "@/lib/store";
import { formatBytes } from "@/lib/format";

const levelLabels = ["Low", "Medium", "High"];

export default function Ready() {
  const navigate = useNavigate();
  const auth = useAppStore((s) => s.auth);
  const earnLevel = useAppStore((s) => s.earnLevel);
  const storageGb = useAppStore((s) => s.storageAllocationGb);
  const setAuth = useAppStore((s) => s.setAuth);

  const levelLabel =
    Number.isInteger(earnLevel) && earnLevel >= 0 && earnLevel <= 2
      ? levelLabels[earnLevel]
      : "Custom";

  const handleEnter = () => {
    setAuth({ setupComplete: true });
    navigate("/");
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
            className="mx-auto w-20 h-20 rounded-2xl bg-[var(--color-accent)] flex items-center justify-center"
          >
            <Check className="h-10 w-10 text-white" />
          </motion.div>
          <h1 className="text-2xl font-bold text-[var(--color-text)]">
            You're All Set
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)]">
            Here's a summary of your choices. You can change any of these in
            settings.
          </p>
        </div>

        {/* Summary cards */}
        <div className="space-y-3">
          <motion.div
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.3 }}
            className="flex items-center gap-4 p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
          >
            <div className="w-10 h-10 rounded-xl bg-[var(--color-accent)]/10 flex items-center justify-center">
              <span className="text-lg font-bold text-[var(--color-accent)]">
                {auth.displayName?.charAt(0).toUpperCase() ?? "?"}
              </span>
            </div>
            <div>
              <p className="text-sm font-semibold text-[var(--color-text)]">
                {auth.displayName}
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                Your display name
              </p>
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.4 }}
            className="flex items-center gap-4 p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
          >
            <div className="w-10 h-10 rounded-xl bg-green-100 dark:bg-green-900/30 flex items-center justify-center">
              <Sprout className="h-5 w-5 text-green-600 dark:text-green-400" />
            </div>
            <div>
              <p className="text-sm font-semibold text-[var(--color-text)]">
                Seeds Wallet
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                Ready to earn and spend
              </p>
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.5 }}
            className="flex items-center gap-4 p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
          >
            <div className="w-10 h-10 rounded-xl bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
              <HardDrive className="h-5 w-5 text-blue-600 dark:text-blue-400" />
            </div>
            <div>
              <p className="text-sm font-semibold text-[var(--color-text)]">
                Earning: {levelLabel}
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                {formatBytes(storageGb * 1024 * 1024 * 1024)} allocated
              </p>
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.6 }}
            className="flex items-center gap-4 p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
          >
            <div className="w-10 h-10 rounded-xl bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center">
              <Shield className="h-5 w-5 text-purple-600 dark:text-purple-400" />
            </div>
            <div>
              <p className="text-sm font-semibold text-[var(--color-text)]">
                Recovery
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                Set up later in settings
              </p>
            </div>
          </motion.div>
        </div>

        <Button className="w-full" size="lg" onClick={handleEnter}>
          Enter Ochra
        </Button>

        {/* Step indicator */}
        <div className="flex justify-center gap-2">
          {[0, 1, 2, 3, 4].map((step) => (
            <div
              key={step}
              className={`w-2 h-2 rounded-full ${
                step === 4 ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
              }`}
            />
          ))}
        </div>
      </motion.div>
    </div>
  );
}
