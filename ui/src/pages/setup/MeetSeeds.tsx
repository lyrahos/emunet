import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Sprout, ArrowRight } from "lucide-react";
import Button from "@/components/ui/Button";

export default function MeetSeeds() {
  const navigate = useNavigate();

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--color-bg)] p-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        className="w-full max-w-md space-y-8 text-center"
      >
        {/* Animated Seed icon */}
        <motion.div
          initial={{ scale: 0, rotate: -180 }}
          animate={{ scale: 1, rotate: 0 }}
          transition={{
            type: "spring",
            stiffness: 150,
            damping: 12,
            delay: 0.3,
          }}
          className="mx-auto w-24 h-24 rounded-3xl bg-green-100 dark:bg-green-900/30 flex items-center justify-center"
        >
          <Sprout className="h-12 w-12 text-green-600 dark:text-green-400" />
        </motion.div>

        <div className="space-y-3">
          <motion.h1
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.5 }}
            className="text-2xl font-bold text-[var(--color-text)]"
          >
            Meet Seeds
          </motion.h1>
          <motion.p
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.7 }}
            className="text-sm text-[var(--color-text-secondary)] leading-relaxed"
          >
            Seeds are the currency of Ochra. You earn them by contributing
            storage and bandwidth to the network. You spend them to access
            content from creators.
          </motion.p>
        </div>

        {/* Feature cards */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.9 }}
          className="space-y-3 text-left"
        >
          <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
            <p className="text-sm font-semibold text-[var(--color-text)]">
              Earn by sharing
            </p>
            <p className="text-xs text-[var(--color-text-secondary)] mt-1">
              Contribute disk space and bandwidth to earn Seeds automatically.
            </p>
          </div>
          <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
            <p className="text-sm font-semibold text-[var(--color-text)]">
              Spend on content
            </p>
            <p className="text-xs text-[var(--color-text-secondary)] mt-1">
              Purchase content from creators inside Spaces. Prices are set in
              Seeds.
            </p>
          </div>
          <div className="p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]">
            <p className="text-sm font-semibold text-[var(--color-text)]">
              Send to anyone
            </p>
            <p className="text-xs text-[var(--color-text-secondary)] mt-1">
              Transfer Seeds directly to friends and creators, no middleman.
            </p>
          </div>
        </motion.div>

        <Button
          className="w-full"
          size="lg"
          onClick={() => navigate("/setup/earn")}
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
                step === 1 ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
              }`}
            />
          ))}
        </div>
      </motion.div>
    </div>
  );
}
