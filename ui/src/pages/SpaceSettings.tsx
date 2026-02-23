import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, AlertTriangle } from "lucide-react";
import Button from "@/components/ui/Button";
import Card from "@/components/ui/Card";
import Modal from "@/components/ui/Modal";
import { rpcCall } from "@/lib/rpc-client";
import { useAppStore } from "@/lib/store";

interface SpaceConfig {
  name: string;
  invitePermission: "host_only" | "creators" | "anyone";
  publishPolicy: "host_only" | "creators" | "anyone";
  revenueSplit: number; // host commission percentage (0-100)
}

export default function SpaceSettings() {
  const { groupId } = useParams<{ groupId: string }>();
  const navigate = useNavigate();
  const addToast = useAppStore((s) => s.addToast);

  const [config, setConfig] = useState<SpaceConfig>({
    name: "",
    invitePermission: "host_only",
    publishPolicy: "creators",
    revenueSplit: 5,
  });
  const [transferOpen, setTransferOpen] = useState(false);
  const [transferTo, setTransferTo] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!groupId) return;
    rpcCall<SpaceConfig>("get_group_settings", { group_id: groupId })
      .then(setConfig)
      .catch(() => {});
  }, [groupId]);

  const handleSave = async () => {
    setSaving(true);
    try {
      await rpcCall("update_group_settings", {
        group_id: groupId,
        ...config,
      });
      addToast({ type: "success", message: "Settings saved." });
    } catch {
      addToast({ type: "error", message: "Failed to save settings." });
    } finally {
      setSaving(false);
    }
  };

  const handleTransferOwnership = async () => {
    if (!transferTo.trim()) return;
    try {
      await rpcCall("transfer_group_ownership", {
        group_id: groupId,
        new_owner_id: transferTo.trim(),
      });
      addToast({
        type: "success",
        message: "Ownership transferred. You are now a Creator.",
      });
      setTransferOpen(false);
      navigate(`/space/${groupId}`);
    } catch {
      addToast({ type: "error", message: "Transfer failed." });
    }
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">
        {/* Header */}
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate(`/space/${groupId}`)}
            className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
          <h1 className="text-xl font-bold text-[var(--color-text)]">
            Space Settings
          </h1>
        </div>

        {/* Name */}
        <Card padding="lg">
          <label className="block text-sm font-medium text-[var(--color-text)] mb-2">
            Space Name
          </label>
          <input
            type="text"
            value={config.name}
            onChange={(e) => setConfig({ ...config, name: e.target.value })}
            maxLength={48}
            className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
          />
        </Card>

        {/* Invite permission */}
        <Card padding="lg">
          <label className="block text-sm font-medium text-[var(--color-text)] mb-2">
            Who can invite members?
          </label>
          <div className="space-y-2">
            {(
              [
                { value: "host_only", label: "Host only" },
                { value: "creators", label: "Creators and above" },
                { value: "anyone", label: "Any member" },
              ] as const
            ).map((option) => (
              <label
                key={option.value}
                className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] cursor-pointer hover:bg-[var(--color-border)]/10 transition-colors"
              >
                <input
                  type="radio"
                  name="invitePermission"
                  value={option.value}
                  checked={config.invitePermission === option.value}
                  onChange={() =>
                    setConfig({ ...config, invitePermission: option.value })
                  }
                  className="w-4 h-4 text-[var(--color-accent)]"
                />
                <span className="text-sm text-[var(--color-text)]">
                  {option.label}
                </span>
              </label>
            ))}
          </div>
        </Card>

        {/* Publish policy */}
        <Card padding="lg">
          <label className="block text-sm font-medium text-[var(--color-text)] mb-2">
            Who can publish content?
          </label>
          <div className="space-y-2">
            {(
              [
                { value: "host_only", label: "Host only" },
                { value: "creators", label: "Creators and above" },
                { value: "anyone", label: "Any member" },
              ] as const
            ).map((option) => (
              <label
                key={option.value}
                className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] cursor-pointer hover:bg-[var(--color-border)]/10 transition-colors"
              >
                <input
                  type="radio"
                  name="publishPolicy"
                  value={option.value}
                  checked={config.publishPolicy === option.value}
                  onChange={() =>
                    setConfig({ ...config, publishPolicy: option.value })
                  }
                  className="w-4 h-4 text-[var(--color-accent)]"
                />
                <span className="text-sm text-[var(--color-text)]">
                  {option.label}
                </span>
              </label>
            ))}
          </div>
        </Card>

        {/* Revenue split */}
        <Card padding="lg">
          <label className="block text-sm font-medium text-[var(--color-text)] mb-2">
            Host Commission
          </label>
          <p className="text-xs text-[var(--color-text-secondary)] mb-3">
            Percentage of each sale that goes to you as the Host.
          </p>
          <div className="flex items-center gap-4">
            <input
              type="range"
              min={0}
              max={50}
              value={config.revenueSplit}
              onChange={(e) =>
                setConfig({
                  ...config,
                  revenueSplit: parseInt(e.target.value, 10),
                })
              }
              className="flex-1 h-2 rounded-full appearance-none bg-[var(--color-border)]
                [&::-webkit-slider-thumb]:appearance-none
                [&::-webkit-slider-thumb]:w-5
                [&::-webkit-slider-thumb]:h-5
                [&::-webkit-slider-thumb]:rounded-full
                [&::-webkit-slider-thumb]:bg-[var(--color-accent)]
                [&::-webkit-slider-thumb]:cursor-pointer"
            />
            <span className="text-sm font-semibold text-[var(--color-text)] min-w-[3rem] text-right">
              {config.revenueSplit}%
            </span>
          </div>
        </Card>

        {/* Save */}
        <Button
          className="w-full"
          size="lg"
          loading={saving}
          onClick={handleSave}
        >
          Save Settings
        </Button>

        {/* Danger zone */}
        <Card padding="lg" className="!border-red-200 dark:!border-red-900/50">
          <div className="flex items-center gap-3 mb-4">
            <AlertTriangle className="h-5 w-5 text-red-500" />
            <h2 className="text-base font-semibold text-red-600 dark:text-red-400">
              Danger Zone
            </h2>
          </div>
          <Button
            variant="danger"
            onClick={() => setTransferOpen(true)}
          >
            Transfer Ownership
          </Button>
        </Card>
      </div>

      {/* Transfer Modal */}
      <Modal
        open={transferOpen}
        onClose={() => setTransferOpen(false)}
        title="Transfer Ownership"
      >
        <div className="space-y-4">
          <p className="text-sm text-[var(--color-text-secondary)]">
            This will make another member the Host of this Space. You will be
            demoted to Creator. This action cannot be undone.
          </p>
          <input
            type="text"
            value={transferTo}
            onChange={(e) => setTransferTo(e.target.value)}
            placeholder="Enter member ID"
            className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] focus:outline-none focus:ring-2 focus:ring-red-500/50 text-sm"
          />
          <div className="flex gap-3">
            <Button
              variant="ghost"
              className="flex-1"
              onClick={() => setTransferOpen(false)}
            >
              Cancel
            </Button>
            <Button
              variant="danger"
              className="flex-1"
              disabled={!transferTo.trim()}
              onClick={handleTransferOwnership}
            >
              Transfer
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
