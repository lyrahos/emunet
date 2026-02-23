import { useState } from "react";
import { motion } from "framer-motion";
import {
  Sprout,
  ArrowUpRight,
  ArrowDownLeft,
  Clock,
  Send,
  QrCode,
  Copy,
  Check,
} from "lucide-react";
import Button from "@/components/ui/Button";
import Modal from "@/components/ui/Modal";
import { useBalance } from "@/hooks/useBalance";
import { useIpc } from "@/hooks/useIpc";
import { useAppStore } from "@/lib/store";
import { formatSeeds, formatRelativeTime } from "@/lib/format";

interface Transaction {
  id: string;
  type: "incoming" | "outgoing";
  amount: number; // micro-seeds
  counterparty: string;
  description: string;
  timestamp: number;
}

export default function Seeds() {
  const { balance, pendingIncoming } = useBalance();
  const { call, loading } = useIpc();
  const addToast = useAppStore((s) => s.addToast);

  const [sendOpen, setSendOpen] = useState(false);
  const [receiveOpen, setReceiveOpen] = useState(false);
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");
  const [copied, setCopied] = useState(false);
  const [transactions] = useState<Transaction[]>([]);

  const handleSend = async () => {
    const microSeeds = Math.floor(parseFloat(sendAmount) * 100_000_000);
    if (isNaN(microSeeds) || microSeeds <= 0) return;

    const result = await call("send_seeds", {
      recipient: sendTo.trim(),
      amount: microSeeds,
    });

    if (result) {
      addToast({
        type: "success",
        message: `Sent ${formatSeeds(microSeeds)} to ${sendTo.trim()}`,
      });
      setSendOpen(false);
      setSendTo("");
      setSendAmount("");
    }
  };

  const handleCopyAddress = () => {
    navigator.clipboard.writeText("ochra:your-receive-address-here");
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">
        {/* Balance card */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-6 rounded-2xl bg-gradient-to-br from-[var(--color-accent)] to-orange-700 text-white"
        >
          <div className="flex items-center gap-2 mb-1">
            <Sprout className="h-5 w-5 opacity-80" />
            <span className="text-sm opacity-80">Your Balance</span>
          </div>
          <p className="text-3xl font-bold">{formatSeeds(balance)}</p>
          {pendingIncoming > 0 && (
            <p className="text-sm mt-1 opacity-70">
              +{formatSeeds(pendingIncoming)} pending
            </p>
          )}

          <div className="flex gap-3 mt-6">
            <Button
              variant="secondary"
              className="flex-1 !bg-white/20 !border-white/30 !text-white hover:!bg-white/30"
              onClick={() => setSendOpen(true)}
            >
              <Send className="h-4 w-4" />
              Send
            </Button>
            <Button
              variant="secondary"
              className="flex-1 !bg-white/20 !border-white/30 !text-white hover:!bg-white/30"
              onClick={() => setReceiveOpen(true)}
            >
              <QrCode className="h-4 w-4" />
              Receive
            </Button>
          </div>
        </motion.div>

        {/* Transaction history */}
        <div>
          <h2 className="text-lg font-semibold text-[var(--color-text)] mb-4">
            Transaction History
          </h2>
          {transactions.length > 0 ? (
            <div className="space-y-2">
              {transactions.map((tx, i) => (
                <motion.div
                  key={tx.id}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: i * 0.03 }}
                  className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
                >
                  <div
                    className={`w-9 h-9 rounded-full flex items-center justify-center ${
                      tx.type === "incoming"
                        ? "bg-green-100 dark:bg-green-900/30"
                        : "bg-red-100 dark:bg-red-900/30"
                    }`}
                  >
                    {tx.type === "incoming" ? (
                      <ArrowDownLeft className="h-4 w-4 text-green-600 dark:text-green-400" />
                    ) : (
                      <ArrowUpRight className="h-4 w-4 text-red-600 dark:text-red-400" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-[var(--color-text)] truncate">
                      {tx.description}
                    </p>
                    <p className="text-xs text-[var(--color-text-secondary)]">
                      {tx.counterparty}
                    </p>
                  </div>
                  <div className="text-right">
                    <p
                      className={`text-sm font-semibold ${
                        tx.type === "incoming"
                          ? "text-green-600 dark:text-green-400"
                          : "text-[var(--color-text)]"
                      }`}
                    >
                      {tx.type === "incoming" ? "+" : "-"}
                      {formatSeeds(tx.amount)}
                    </p>
                    <p className="text-xs text-[var(--color-text-secondary)]">
                      {formatRelativeTime(tx.timestamp)}
                    </p>
                  </div>
                </motion.div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center py-12 text-center">
              <Clock className="h-10 w-10 text-[var(--color-text-secondary)]/30 mb-3" />
              <p className="text-sm text-[var(--color-text-secondary)]">
                No transactions yet
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]/60 mt-1">
                Start earning Seeds by contributing to the network.
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Send Modal */}
      <Modal open={sendOpen} onClose={() => setSendOpen(false)} title="Send Seeds">
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-[var(--color-text)] mb-1.5">
              Recipient
            </label>
            <input
              type="text"
              value={sendTo}
              onChange={(e) => setSendTo(e.target.value)}
              placeholder="Contact ID or address"
              className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-[var(--color-text)] mb-1.5">
              Amount (Seeds)
            </label>
            <input
              type="number"
              value={sendAmount}
              onChange={(e) => setSendAmount(e.target.value)}
              placeholder="0.00"
              min="0"
              step="0.01"
              className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
            />
            <p className="mt-1 text-xs text-[var(--color-text-secondary)]">
              Available: {formatSeeds(balance)}
            </p>
          </div>
          <Button
            className="w-full"
            loading={loading}
            disabled={!sendTo.trim() || !sendAmount || parseFloat(sendAmount) <= 0}
            onClick={handleSend}
          >
            <Send className="h-4 w-4" />
            Send Seeds
          </Button>
        </div>
      </Modal>

      {/* Receive Modal */}
      <Modal
        open={receiveOpen}
        onClose={() => setReceiveOpen(false)}
        title="Receive Seeds"
      >
        <div className="space-y-4 text-center">
          <div className="mx-auto w-48 h-48 rounded-xl bg-[var(--color-border)]/30 flex items-center justify-center">
            <QrCode className="h-24 w-24 text-[var(--color-text-secondary)]/40" />
          </div>
          <p className="text-xs text-[var(--color-text-secondary)]">
            Share this address or QR code to receive Seeds.
          </p>
          <Button
            variant="secondary"
            className="w-full"
            onClick={handleCopyAddress}
          >
            {copied ? (
              <Check className="h-4 w-4 text-green-500" />
            ) : (
              <Copy className="h-4 w-4" />
            )}
            {copied ? "Copied!" : "Copy Address"}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
