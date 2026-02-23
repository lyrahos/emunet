import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import {
  Search,
  UserPlus,
  MessageCircle,
  QrCode,
  Share2,
  Copy,
  Check,
} from "lucide-react";
import Button from "@/components/ui/Button";
import Modal from "@/components/ui/Modal";
import { rpcCall } from "@/lib/rpc-client";
import { useAppStore } from "@/lib/store";

interface Contact {
  id: string;
  displayName: string;
  pikHash: string;
  online: boolean;
  lastSeen: number;
}

export default function Contacts() {
  const navigate = useNavigate();
  const addToast = useAppStore((s) => s.addToast);

  const [contacts, setContacts] = useState<Contact[]>([]);
  const [query, setQuery] = useState("");
  const [addOpen, setAddOpen] = useState(false);
  const [shareCode, setShareCode] = useState("");
  const [inputCode, setInputCode] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    rpcCall<Contact[]>("get_contacts")
      .then(setContacts)
      .catch(() => {});
  }, []);

  useEffect(() => {
    rpcCall<{ invite_code: string }>("get_my_invite_code")
      .then((res) => setShareCode(res.invite_code))
      .catch(() => {});
  }, []);

  const filteredContacts = query.trim()
    ? contacts.filter((c) =>
        c.displayName.toLowerCase().includes(query.toLowerCase()),
      )
    : contacts;

  const handleAddContact = async () => {
    if (!inputCode.trim()) return;
    try {
      const res = await rpcCall<Contact>("add_contact", {
        invite_code: inputCode.trim(),
      });
      setContacts((prev) => [...prev, res]);
      setInputCode("");
      setAddOpen(false);
      addToast({ type: "success", message: `Added ${res.displayName}` });
    } catch {
      addToast({ type: "error", message: "Invalid invite code." });
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(shareCode);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center justify-between px-6 pt-6 pb-4">
        <h1 className="text-xl font-bold text-[var(--color-text)]">
          Contacts
        </h1>
        <Button size="sm" onClick={() => setAddOpen(true)}>
          <UserPlus className="h-4 w-4" />
          Add Contact
        </Button>
      </header>

      {/* Search */}
      <div className="px-6 pb-4">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-[var(--color-text-secondary)]" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search contacts..."
            className="w-full pl-10 pr-4 py-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
          />
        </div>
      </div>

      {/* Contact list */}
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {filteredContacts.length > 0 ? (
          <div className="space-y-2">
            <AnimatePresence>
              {filteredContacts.map((contact, i) => (
                <motion.div
                  key={contact.id}
                  initial={{ opacity: 0, y: 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.03 }}
                  className="flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]"
                >
                  <div className="relative">
                    <div className="w-10 h-10 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center">
                      <span className="text-sm font-semibold text-[var(--color-accent)]">
                        {contact.displayName.charAt(0).toUpperCase()}
                      </span>
                    </div>
                    {contact.online && (
                      <div className="absolute bottom-0 right-0 w-3 h-3 rounded-full bg-green-500 border-2 border-[var(--color-surface)]" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-semibold text-[var(--color-text)] truncate">
                      {contact.displayName}
                    </p>
                    <p className="text-xs text-[var(--color-text-secondary)]">
                      {contact.online ? "Online" : "Offline"}
                    </p>
                  </div>
                  <button
                    onClick={() => navigate(`/whisper/${contact.id}`)}
                    className="p-2 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors"
                    title="Whisper"
                  >
                    <MessageCircle className="h-5 w-5" />
                  </button>
                </motion.div>
              ))}
            </AnimatePresence>
          </div>
        ) : contacts.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center space-y-4">
            <div className="w-16 h-16 rounded-2xl bg-[var(--color-accent)]/10 flex items-center justify-center">
              <UserPlus className="h-8 w-8 text-[var(--color-accent)]" />
            </div>
            <div>
              <p className="text-base font-semibold text-[var(--color-text)]">
                No Contacts Yet
              </p>
              <p className="text-sm text-[var(--color-text-secondary)] mt-1">
                Share your invite code or scan someone else's to connect.
              </p>
            </div>
            <Button onClick={() => setAddOpen(true)}>
              <UserPlus className="h-4 w-4" />
              Add Contact
            </Button>
          </div>
        ) : (
          <div className="flex flex-col items-center py-12">
            <p className="text-sm text-[var(--color-text-secondary)]">
              No contacts match "{query}"
            </p>
          </div>
        )}
      </div>

      {/* Add Contact Modal */}
      <Modal
        open={addOpen}
        onClose={() => setAddOpen(false)}
        title="Add Contact"
      >
        <div className="space-y-6">
          {/* Your code */}
          <div className="space-y-3">
            <h3 className="text-sm font-semibold text-[var(--color-text)]">
              Your Invite Code
            </h3>
            <div className="flex items-center justify-center py-6 rounded-xl bg-[var(--color-border)]/20">
              <QrCode className="h-20 w-20 text-[var(--color-text-secondary)]/40" />
            </div>
            <div className="flex gap-2">
              <Button
                variant="secondary"
                className="flex-1"
                onClick={handleCopy}
              >
                {copied ? (
                  <Check className="h-4 w-4 text-green-500" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
                {copied ? "Copied!" : "Copy Code"}
              </Button>
              <Button variant="secondary" className="flex-1">
                <Share2 className="h-4 w-4" />
                Share
              </Button>
            </div>
          </div>

          <div className="border-t border-[var(--color-border)]" />

          {/* Add by code */}
          <div className="space-y-3">
            <h3 className="text-sm font-semibold text-[var(--color-text)]">
              Add Someone
            </h3>
            <input
              type="text"
              value={inputCode}
              onChange={(e) => setInputCode(e.target.value)}
              placeholder="Paste their invite code"
              className="w-full px-4 py-2.5 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
            />
            <Button
              className="w-full"
              disabled={!inputCode.trim()}
              onClick={handleAddContact}
            >
              Add Contact
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
