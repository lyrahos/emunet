import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { Eye, EyeOff } from "lucide-react";
import Button from "@/components/ui/Button";
import { useIpc } from "@/hooks/useIpc";
import { useAppStore } from "@/lib/store";

export default function Welcome() {
  const navigate = useNavigate();
  const { call, loading } = useIpc();
  const setAuth = useAppStore((s) => s.setAuth);

  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState("");

  const canProceed =
    name.trim().length >= 2 &&
    password.length >= 8 &&
    password === confirmPassword;

  const handleCreate = async () => {
    if (!canProceed) return;
    setError("");

    const result = await call("init_pik", { password });

    if (result) {
      // Session is now unlocked — save display name
      await call("update_display_name", { new_name: name.trim() });

      setAuth({
        unlocked: true,
        displayName: name.trim(),
        pikHash: (result as { pik_hash: string }).pik_hash,
      });
      navigate("/setup/seeds");
    } else {
      setError("Failed to create identity. Please try again.");
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--color-bg)] p-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        className="w-full max-w-md space-y-8"
      >
        {/* Logo and welcome */}
        <div className="text-center space-y-3">
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            transition={{ type: "spring", stiffness: 200, damping: 15, delay: 0.2 }}
            className="mx-auto w-20 h-20 rounded-2xl bg-[var(--color-accent)] flex items-center justify-center"
          >
            <span className="text-3xl font-bold text-white">O</span>
          </motion.div>
          <h1 className="text-2xl font-bold text-[var(--color-text)]">
            Welcome to Ochra
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)]">
            Your private, decentralized content network.
            <br />
            Let's create your identity.
          </p>
        </div>

        {/* Form */}
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-[var(--color-text)] mb-1.5">
              Display Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Choose a name others will see"
              maxLength={32}
              className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50"
            />
            <p className="mt-1 text-xs text-[var(--color-text-secondary)]">
              2-32 characters. You can change this later.
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-[var(--color-text)] mb-1.5">
              Password
            </label>
            <div className="relative">
              <input
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="At least 8 characters"
                className="w-full px-4 py-2.5 pr-10 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50"
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-[var(--color-text-secondary)] hover:text-[var(--color-text)]"
              >
                {showPassword ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
              </button>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-[var(--color-text)] mb-1.5">
              Confirm Password
            </label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="Type your password again"
              className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50"
            />
            {confirmPassword.length > 0 && password !== confirmPassword && (
              <p className="mt-1 text-xs text-red-500">
                Passwords do not match.
              </p>
            )}
          </div>

          {error && (
            <p className="text-sm text-red-500 text-center">{error}</p>
          )}

          <Button
            className="w-full"
            size="lg"
            loading={loading}
            disabled={!canProceed}
            onClick={handleCreate}
          >
            Create Identity
          </Button>
        </div>

        {/* Step indicator */}
        <div className="flex justify-center gap-2">
          {[0, 1, 2, 3, 4].map((step) => (
            <div
              key={step}
              className={`w-2 h-2 rounded-full ${
                step === 0 ? "bg-[var(--color-accent)]" : "bg-[var(--color-border)]"
              }`}
            />
          ))}
        </div>
      </motion.div>
    </div>
  );
}
