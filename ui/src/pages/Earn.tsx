import { motion } from "framer-motion";
import {
  HardDrive,
  Activity,
  TrendingUp,
  Database,
  Wifi,
} from "lucide-react";
import Card from "@/components/ui/Card";
import Slider from "@/components/ui/Slider";
import { useAppStore } from "@/lib/store";
import { formatBytes } from "@/lib/format";

const levels = [
  { label: "Low", emoji: "\u{1F331}", description: "~2 GB, minimal bandwidth" },
  { label: "Medium", emoji: "\u{1F33F}", description: "~10 GB, moderate bandwidth" },
  { label: "High", emoji: "\u{1F333}", description: "~50 GB, generous bandwidth" },
];

const storageForLevel = (level: number): number => {
  if (level <= 0) return 2;
  if (level >= 2) return 50;
  if (level <= 1) return 2 + (10 - 2) * level;
  return 10 + (50 - 10) * (level - 1);
};

export default function Earn() {
  const earnLevel = useAppStore((s) => s.earnLevel);
  const setEarnLevel = useAppStore((s) => s.setEarnLevel);
  const setStorageAllocation = useAppStore((s) => s.setStorageAllocation);
  const storageAllocationGb = useAppStore((s) => s.storageAllocationGb);

  const handleLevelChange = (value: number) => {
    setEarnLevel(value);
    setStorageAllocation(storageForLevel(value));
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">
        <div>
          <h1 className="text-xl font-bold text-[var(--color-text)]">
            Earn Settings
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)] mt-1">
            Control how you contribute to the Ochra network and earn Seeds.
          </p>
        </div>

        {/* Stats cards */}
        <div className="grid grid-cols-3 gap-3">
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
          >
            <Card padding="md">
              <div className="flex items-center gap-2 mb-2">
                <Database className="h-4 w-4 text-blue-500" />
                <span className="text-xs text-[var(--color-text-secondary)]">
                  Storage Used
                </span>
              </div>
              <p className="text-lg font-bold text-[var(--color-text)]">
                {formatBytes(0)}
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                of {formatBytes(storageAllocationGb * 1024 * 1024 * 1024)}
              </p>
            </Card>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.15 }}
          >
            <Card padding="md">
              <div className="flex items-center gap-2 mb-2">
                <Wifi className="h-4 w-4 text-green-500" />
                <span className="text-xs text-[var(--color-text-secondary)]">
                  Bandwidth
                </span>
              </div>
              <p className="text-lg font-bold text-[var(--color-text)]">
                {formatBytes(0)}/s
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                current upload
              </p>
            </Card>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
          >
            <Card padding="md">
              <div className="flex items-center gap-2 mb-2">
                <TrendingUp className="h-4 w-4 text-[var(--color-accent)]" />
                <span className="text-xs text-[var(--color-text-secondary)]">
                  Earning Rate
                </span>
              </div>
              <p className="text-lg font-bold text-[var(--color-text)]">
                0.00
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                Seeds / hour
              </p>
            </Card>
          </motion.div>
        </div>

        {/* Earning level */}
        <Card padding="lg">
          <div className="flex items-center gap-3 mb-5">
            <div className="w-10 h-10 rounded-xl bg-[var(--color-accent)]/10 flex items-center justify-center">
              <HardDrive className="h-5 w-5 text-[var(--color-accent)]" />
            </div>
            <div>
              <h2 className="text-base font-semibold text-[var(--color-text)]">
                Earning Level
              </h2>
              <p className="text-xs text-[var(--color-text-secondary)]">
                Adjust your contribution level
              </p>
            </div>
          </div>
          <Slider
            value={earnLevel}
            onChange={handleLevelChange}
            levels={levels}
            showCustom
          />
        </Card>

        {/* Storage allocation detail */}
        <Card padding="lg">
          <h2 className="text-base font-semibold text-[var(--color-text)] mb-4">
            Storage Allocation
          </h2>
          <div className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-[var(--color-text-secondary)]">
                Allocated
              </span>
              <span className="font-semibold text-[var(--color-text)]">
                {formatBytes(storageAllocationGb * 1024 * 1024 * 1024)}
              </span>
            </div>
            <div className="w-full h-3 rounded-full bg-[var(--color-border)]">
              <motion.div
                className="h-full rounded-full bg-blue-500"
                animate={{ width: "0%" }}
                transition={{ type: "spring", stiffness: 200, damping: 25 }}
              />
            </div>
            <div className="flex items-center justify-between text-xs text-[var(--color-text-secondary)]">
              <span>0 B used</span>
              <span>
                {formatBytes(storageAllocationGb * 1024 * 1024 * 1024)} free
              </span>
            </div>
          </div>
        </Card>

        {/* ABR Telemetry */}
        <Card padding="lg">
          <div className="flex items-center gap-3 mb-3">
            <Activity className="h-5 w-5 text-[var(--color-text-secondary)]" />
            <div>
              <h2 className="text-base font-semibold text-[var(--color-text)]">
                Contribution Metrics
              </h2>
              <p className="text-xs text-[var(--color-text-secondary)]">
                Anonymous bandwidth and reliability telemetry
              </p>
            </div>
          </div>
          <div className="space-y-2">
            <div className="flex items-center justify-between py-2 border-b border-[var(--color-border)]">
              <span className="text-sm text-[var(--color-text-secondary)]">
                Availability
              </span>
              <span className="text-sm font-medium text-[var(--color-text)]">
                --
              </span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-[var(--color-border)]">
              <span className="text-sm text-[var(--color-text-secondary)]">
                Bandwidth Score
              </span>
              <span className="text-sm font-medium text-[var(--color-text)]">
                --
              </span>
            </div>
            <div className="flex items-center justify-between py-2">
              <span className="text-sm text-[var(--color-text-secondary)]">
                Reliability
              </span>
              <span className="text-sm font-medium text-[var(--color-text)]">
                --
              </span>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}
