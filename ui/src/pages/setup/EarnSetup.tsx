import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { HardDrive, ArrowRight } from "lucide-react";
import Button from "@/components/ui/Button";
import Slider from "@/components/ui/Slider";
import { useAppStore } from "@/lib/store";
import { formatBytes } from "@/lib/format";

const levels = [
  { label: "Low", emoji: "\u{1F331}", description: "~2 GB storage, minimal bandwidth" },
  { label: "Medium", emoji: "\u{1F33F}", description: "~10 GB storage, moderate bandwidth" },
  { label: "High", emoji: "\u{1F333}", description: "~50 GB storage, generous bandwidth" },
];

const storageForLevel = (level: number): number => {
  // Interpolate between levels: 0->2GB, 1->10GB, 2->50GB
  if (level <= 0) return 2;
  if (level >= 2) return 50;
  if (level <= 1) return 2 + (10 - 2) * level;
  return 10 + (50 - 10) * (level - 1);
};

export default function EarnSetup() {
  const navigate = useNavigate();
  const earnLevel = useAppStore((s) => s.earnLevel);
  const setEarnLevel = useAppStore((s) => s.setEarnLevel);
  const setStorageAllocation = useAppStore((s) => s.setStorageAllocation);

  const handleLevelChange = (value: number) => {
    setEarnLevel(value);
    setStorageAllocation(storageForLevel(value));
  };

  const storageGb = storageForLevel(earnLevel);

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
            className="mx-auto w-16 h-16 rounded-2xl bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center"
          >
            <HardDrive className="h-8 w-8 text-blue-600 dark:text-blue-400" />
          </motion.div>
          <h1 className="text-2xl font-bold text-[var(--color-text)]">
            Set Your Earning Level
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)]">
            Choose how much you want to contribute to the network. More
            contribution means more Seeds earned.
          </p>
        </div>

        {/* Slider */}
        <div className="p-6 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
          <Slider
            value={earnLevel}
            onChange={handleLevelChange}
            levels={levels}
            showCustom
          />
        </div>

        {/* Storage detail */}
        <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
          <div className="flex items-center justify-between">
            <span className="text-sm text-[var(--color-text-secondary)]">
              Disk space allocation
            </span>
            <span className="text-sm font-semibold text-[var(--color-text)]">
              {formatBytes(storageGb * 1024 * 1024 * 1024)}
            </span>
          </div>
          <div className="mt-3 w-full h-2 rounded-full bg-[var(--color-border)]">
            <motion.div
              className="h-full rounded-full bg-[var(--color-accent)]"
              initial={{ width: 0 }}
              animate={{ width: `${(storageGb / 50) * 100}%` }}
              transition={{ type: "spring", stiffness: 200, damping: 25 }}
            />
          </div>
          <p className="mt-2 text-xs text-[var(--color-text-secondary)]">
            You can change this at any time in your settings.
          </p>
        </div>

        <Button
          className="w-full"
          size="lg"
          onClick={() => navigate("/setup/recovery")}
        >
          Continue
          <ArrowRight className="h-4 w-4" />
        </Button>

        {/* Step indicator */}
        <div className="flex justify-center gap-2">
          {[0, 1, 2, 3, 4].map((step) => (
            <div
              key={step}
              className={`w-2 h-2 rounded-full ${
                step === 2 ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
              }`}
            />
          ))}
        </div>
      </motion.div>
    </div>
  );
}
