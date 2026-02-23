import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import { MessageCircle, Plus } from "lucide-react";
import Button from "@/components/ui/Button";
import Modal from "@/components/ui/Modal";
import { rpcCall } from "@/lib/rpc-client";
import { formatRelativeTime } from "@/lib/format";

interface WhisperSession {
  sessionId: string;
  contactName: string;
  lastMessage: string;
  lastMessageTime: number;
  unreadCount: number;
  online: boolean;
}

interface ContactOption {
  id: string;
  displayName: string;
}

export default function Whisper() {
  const navigate = useNavigate();
  const [sessions, setSessions] = useState<WhisperSession[]>([]);
  const [newOpen, setNewOpen] = useState(false);
  const [contacts, setContacts] = useState<ContactOption[]>([]);

  useEffect(() => {
    rpcCall<WhisperSession[]>("get_whisper_sessions")
      .then(setSessions)
      .catch(() => {});
  }, []);

  const handleNewWhisper = async (contactId: string) => {
    try {
      const res = await rpcCall<{ session_id: string }>(
        "create_whisper_session",
        { contact_id: contactId },
      );
      setNewOpen(false);
      navigate(`/whisper/${res.session_id}`);
    } catch {
      // Could not create session
    }
  };

  const openNewWhisperModal = async () => {
    try {
      const c = await rpcCall<ContactOption[]>("get_contacts");
      setContacts(c);
    } catch {
      setContacts([]);
    }
    setNewOpen(true);
  };

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center justify-between px-6 pt-6 pb-4">
        <h1 className="text-xl font-bold text-[var(--color-text)]">Whisper</h1>
        <Button size="sm" onClick={openNewWhisperModal}>
          <Plus className="h-4 w-4" />
          New Whisper
        </Button>
      </header>

      {/* Session list */}
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {sessions.length > 0 ? (
          <div className="space-y-2">
            <AnimatePresence>
              {sessions.map((session, i) => (
                <motion.button
                  key={session.sessionId}
                  initial={{ opacity: 0, y: 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.03 }}
                  whileHover={{ x: 2 }}
                  onClick={() => navigate(`/whisper/${session.sessionId}`)}
                  className="w-full text-left flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:shadow-sm transition-shadow"
                >
                  <div className="relative">
                    <div className="w-10 h-10 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center">
                      <span className="text-sm font-semibold text-[var(--color-accent)]">
                        {session.contactName.charAt(0).toUpperCase()}
                      </span>
                    </div>
                    {session.online && (
                      <div className="absolute bottom-0 right-0 w-3 h-3 rounded-full bg-green-500 border-2 border-[var(--color-surface)]" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between">
                      <p className="text-sm font-semibold text-[var(--color-text)] truncate">
                        {session.contactName}
                      </p>
                      <span className="text-xs text-[var(--color-text-secondary)] flex-shrink-0 ml-2">
                        {formatRelativeTime(session.lastMessageTime)}
                      </span>
                    </div>
                    <div className="flex items-center justify-between mt-0.5">
                      <p className="text-xs text-[var(--color-text-secondary)] truncate">
                        {session.lastMessage}
                      </p>
                      {session.unreadCount > 0 && (
                        <span className="ml-2 flex-shrink-0 w-5 h-5 rounded-full bg-[var(--color-accent)] flex items-center justify-center text-[10px] font-bold text-white">
                          {session.unreadCount > 9
                            ? "9+"
                            : session.unreadCount}
                        </span>
                      )}
                    </div>
                  </div>
                </motion.button>
              ))}
            </AnimatePresence>
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-center space-y-4">
            <div className="w-16 h-16 rounded-2xl bg-[var(--color-accent)]/10 flex items-center justify-center">
              <MessageCircle className="h-8 w-8 text-[var(--color-accent)]" />
            </div>
            <div>
              <p className="text-base font-semibold text-[var(--color-text)]">
                No Whisper Sessions
              </p>
              <p className="text-sm text-[var(--color-text-secondary)] mt-1">
                Start an end-to-end encrypted conversation with a contact.
              </p>
            </div>
            <Button onClick={openNewWhisperModal}>
              <Plus className="h-4 w-4" />
              New Whisper
            </Button>
          </div>
        )}
      </div>

      {/* New Whisper Modal */}
      <Modal
        open={newOpen}
        onClose={() => setNewOpen(false)}
        title="New Whisper"
      >
        <div className="space-y-3">
          <p className="text-sm text-[var(--color-text-secondary)]">
            Choose a contact to start a Whisper with.
          </p>
          {contacts.length > 0 ? (
            contacts.map((c) => (
              <button
                key={c.id}
                onClick={() => handleNewWhisper(c.id)}
                className="w-full flex items-center gap-3 p-3 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:bg-[var(--color-border)]/20 transition-colors text-left"
              >
                <div className="w-9 h-9 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center">
                  <span className="text-sm font-semibold text-[var(--color-accent)]">
                    {c.displayName.charAt(0).toUpperCase()}
                  </span>
                </div>
                <span className="text-sm font-medium text-[var(--color-text)]">
                  {c.displayName}
                </span>
              </button>
            ))
          ) : (
            <p className="text-sm text-[var(--color-text-secondary)] text-center py-4">
              No contacts yet. Add a contact first.
            </p>
          )}
        </div>
      </Modal>
    </div>
  );
}
