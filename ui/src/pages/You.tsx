import { useState } from "react";
import { motion } from "framer-motion";
import {
  User,
  Moon,
  Sun,
  Monitor,
  Bell,
  Download,
  Lock,
  ChevronRight,
  Pencil,
} from "lucide-react";
import Button from "@/components/ui/Button";
import Card from "@/components/ui/Card";
import { useAppStore } from "@/lib/store";
import { useIpc } from "@/hooks/useIpc";
import { truncateHash } from "@/lib/format";

type Theme = "light" | "dark" | "system";

const themes: { value: Theme; label: string; icon: React.ElementType }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

export default function You() {
  const auth = useAppStore((s) => s.auth);
  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);
  const setAuth = useAppStore((s) => s.setAuth);
  const addToast = useAppStore((s) => s.addToast);
  const { call } = useIpc();

  const [editingName, setEditingName] = useState(false);
  const [newName, setNewName] = useState(auth.displayName ?? "");

  const handleSaveName = async () => {
    if (newName.trim().length < 2) return;
    await call("update_display_name", { display_name: newName.trim() });
    setAuth({ displayName: newName.trim() });
    setEditingName(false);
    addToast({ type: "success", message: "Display name updated." });
  };

  const handleLockSession = async () => {
    await call("lock_session", {});
    setAuth({ unlocked: false });
  };

  const handleExportData = async () => {
    await call("export_user_data", {});
    addToast({ type: "info", message: "Data export started. Check your downloads." });
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">
        <h1 className="text-xl font-bold text-[var(--color-text)]">Settings</h1>

        {/* Profile card */}
        <Card padding="lg">
          <div className="flex items-center gap-4">
            <div className="w-16 h-16 rounded-2xl bg-[var(--color-accent)]/10 flex items-center justify-center">
              <User className="h-8 w-8 text-[var(--color-accent)]" />
            </div>
            <div className="flex-1">
              {editingName ? (
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleSaveName()}
                    maxLength={32}
                    className="flex-1 px-3 py-1.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
                    autoFocus
                  />
                  <Button size="sm" onClick={handleSaveName}>
                    Save
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setEditingName(false);
                      setNewName(auth.displayName ?? "");
                    }}
                  >
                    Cancel
                  </Button>
                </div>
              ) : (
                <div className="flex items-center gap-2">
                  <h2 className="text-lg font-semibold text-[var(--color-text)]">
                    {auth.displayName}
                  </h2>
                  <button
                    onClick={() => setEditingName(true)}
                    className="p-1 rounded-md text-[var(--color-text-secondary)] hover:text-[var(--color-text)] transition-colors"
                  >
                    <Pencil className="h-3.5 w-3.5" />
                  </button>
                </div>
              )}
              <p className="text-xs text-[var(--color-text-secondary)] mt-0.5">
                {auth.pikHash ? truncateHash(auth.pikHash) : "Identity"}
              </p>
            </div>
          </div>
        </Card>

        {/* Theme toggle */}
        <Card padding="lg">
          <h2 className="text-base font-semibold text-[var(--color-text)] mb-4">
            Theme
          </h2>
          <div className="grid grid-cols-3 gap-2">
            {themes.map((t) => (
              <motion.button
                key={t.value}
                whileTap={{ scale: 0.95 }}
                onClick={() => setTheme(t.value)}
                className={`flex flex-col items-center gap-2 p-3 rounded-xl border transition-colors ${
                  theme === t.value
                    ? "border-[var(--color-accent)] bg-[var(--color-accent)]/5"
                    : "border-[var(--color-border)] hover:border-[var(--color-accent)]/40"
                }`}
              >
                <t.icon
                  className={`h-5 w-5 ${
                    theme === t.value
                      ? "text-[var(--color-accent)]"
                      : "text-[var(--color-text-secondary)]"
                  }`}
                />
                <span
                  className={`text-xs font-medium ${
                    theme === t.value
                      ? "text-[var(--color-accent)]"
                      : "text-[var(--color-text-secondary)]"
                  }`}
                >
                  {t.label}
                </span>
              </motion.button>
            ))}
          </div>
        </Card>

        {/* Settings list */}
        <Card padding="none">
          <SettingsRow
            icon={Bell}
            label="Notifications"
            description="Manage notification preferences"
          />
          <SettingsRow
            icon={Download}
            label="Export Data"
            description="Download all your data"
            onClick={handleExportData}
          />
          <SettingsRow
            icon={Lock}
            label="Lock Session"
            description="Require password to re-enter"
            onClick={handleLockSession}
            isLast
          />
        </Card>
      </div>
    </div>
  );
}

function SettingsRow({
  icon: Icon,
  label,
  description,
  onClick,
  isLast = false,
}: {
  icon: React.ElementType;
  label: string;
  description: string;
  onClick?: () => void;
  isLast?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={`w-full flex items-center gap-4 px-5 py-4 text-left hover:bg-[var(--color-border)]/20 transition-colors ${
        !isLast ? "border-b border-[var(--color-border)]" : ""
      }`}
    >
      <Icon className="h-5 w-5 text-[var(--color-text-secondary)]" />
      <div className="flex-1">
        <p className="text-sm font-medium text-[var(--color-text)]">{label}</p>
        <p className="text-xs text-[var(--color-text-secondary)]">
          {description}
        </p>
      </div>
      <ChevronRight className="h-4 w-4 text-[var(--color-text-secondary)]" />
    </button>
  );
}
