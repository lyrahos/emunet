import { useState, useEffect, useRef } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowLeft,
  Send,
  Sprout,
  Eye,
  CheckCheck,
  Check as CheckIcon,
} from "lucide-react";
import Button from "@/components/ui/Button";
import { rpcCall } from "@/lib/rpc-client";
import { useEvents } from "@/hooks/useEvents";
import { formatRelativeTime, formatSeeds } from "@/lib/format";

interface Message {
  id: string;
  content: string;
  senderId: string;
  timestamp: number;
  read: boolean;
  type: "text" | "seed_transfer" | "identity_reveal";
  amount?: number; // micro-seeds for seed_transfer
}

interface SessionInfo {
  sessionId: string;
  contactName: string;
  contactId: string;
  online: boolean;
}

export default function WhisperChat() {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const [session, setSession] = useState<SessionInfo | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [typing, setTyping] = useState(false);
  const [myId, setMyId] = useState("");

  useEffect(() => {
    if (!sessionId) return;
    rpcCall<SessionInfo>("get_whisper_session", { session_id: sessionId })
      .then(setSession)
      .catch(() => {});
    rpcCall<Message[]>("get_whisper_messages", { session_id: sessionId })
      .then(setMessages)
      .catch(() => {});
    rpcCall<{ pik_hash: string }>("get_my_identity")
      .then((res) => setMyId(res.pik_hash))
      .catch(() => {});
  }, [sessionId]);

  // Listen for new messages
  useEvents(`whisper:${sessionId}:message`, (payload) => {
    const msg = payload as Message;
    setMessages((prev) => [...prev, msg]);
  });

  // Listen for typing indicator
  useEvents(`whisper:${sessionId}:typing`, () => {
    setTyping(true);
    setTimeout(() => setTyping(false), 3000);
  });

  // Auto-scroll
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, typing]);

  const handleSend = async () => {
    if (!input.trim() || !sessionId) return;
    const text = input.trim();
    setInput("");

    try {
      await rpcCall("send_whisper_message", {
        session_id: sessionId,
        content: text,
        type: "text",
      });
    } catch {
      // Message failed to send
    }
  };

  const handleSendSeeds = async () => {
    const amountStr = prompt("Enter amount of Seeds to send:");
    if (!amountStr) return;
    const microSeeds = Math.floor(parseFloat(amountStr) * 100_000_000);
    if (isNaN(microSeeds) || microSeeds <= 0) return;

    try {
      await rpcCall("send_whisper_message", {
        session_id: sessionId,
        content: `Sent ${formatSeeds(microSeeds)}`,
        type: "seed_transfer",
        amount: microSeeds,
      });
    } catch {
      // Transfer failed
    }
  };

  const handleRevealIdentity = async () => {
    try {
      await rpcCall("send_whisper_message", {
        session_id: sessionId,
        content: "Identity revealed",
        type: "identity_reveal",
      });
    } catch {
      // Reveal failed
    }
  };

  const isMe = (senderId: string) => senderId === myId;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="flex items-center gap-3 px-4 py-3 border-b border-[var(--color-border)] bg-[var(--color-surface)]">
        <button
          onClick={() => navigate("/whisper")}
          className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
        >
          <ArrowLeft className="h-5 w-5" />
        </button>
        <div className="relative">
          <div className="w-9 h-9 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center">
            <span className="text-sm font-semibold text-[var(--color-accent)]">
              {session?.contactName.charAt(0).toUpperCase() ?? "?"}
            </span>
          </div>
          {session?.online && (
            <div className="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full bg-green-500 border-2 border-[var(--color-surface)]" />
          )}
        </div>
        <div className="flex-1">
          <p className="text-sm font-semibold text-[var(--color-text)]">
            {session?.contactName ?? "Loading..."}
          </p>
          <p className="text-xs text-[var(--color-text-secondary)]">
            {session?.online ? "Online" : "Offline"}
          </p>
        </div>
        <div className="flex gap-1">
          <button
            onClick={handleSendSeeds}
            className="p-2 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors"
            title="Send Seeds"
          >
            <Sprout className="h-5 w-5" />
          </button>
          <button
            onClick={handleRevealIdentity}
            className="p-2 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors"
            title="Reveal Identity"
          >
            <Eye className="h-5 w-5" />
          </button>
        </div>
      </header>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-3">
        <AnimatePresence>
          {messages.map((msg) => {
            const mine = isMe(msg.senderId);
            return (
              <motion.div
                key={msg.id}
                initial={{ opacity: 0, y: 10, scale: 0.95 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                transition={{ type: "spring", stiffness: 300, damping: 25 }}
                className={`flex ${mine ? "justify-end" : "justify-start"}`}
              >
                <div
                  className={`max-w-[75%] rounded-2xl px-4 py-2.5 ${
                    mine
                      ? "bg-[var(--color-accent)] text-white rounded-br-md"
                      : "bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-text)] rounded-bl-md"
                  }`}
                >
                  {msg.type === "seed_transfer" && (
                    <div
                      className={`flex items-center gap-1.5 mb-1 ${
                        mine ? "text-white/80" : "text-[var(--color-accent)]"
                      }`}
                    >
                      <Sprout className="h-3.5 w-3.5" />
                      <span className="text-xs font-semibold">
                        {mine ? "You sent" : "Received"}{" "}
                        {formatSeeds(msg.amount ?? 0)}
                      </span>
                    </div>
                  )}

                  {msg.type === "identity_reveal" && (
                    <div
                      className={`flex items-center gap-1.5 mb-1 ${
                        mine ? "text-white/80" : "text-purple-500"
                      }`}
                    >
                      <Eye className="h-3.5 w-3.5" />
                      <span className="text-xs font-semibold">
                        Identity Revealed
                      </span>
                    </div>
                  )}

                  <p className="text-sm leading-relaxed">{msg.content}</p>

                  <div
                    className={`flex items-center gap-1 mt-1 ${
                      mine ? "justify-end" : "justify-start"
                    }`}
                  >
                    <span
                      className={`text-[10px] ${
                        mine
                          ? "text-white/60"
                          : "text-[var(--color-text-secondary)]"
                      }`}
                    >
                      {formatRelativeTime(msg.timestamp)}
                    </span>
                    {mine && (
                      msg.read ? (
                        <CheckCheck
                          className="h-3 w-3 text-white/60"
                        />
                      ) : (
                        <CheckIcon
                          className="h-3 w-3 text-white/40"
                        />
                      )
                    )}
                  </div>
                </div>
              </motion.div>
            );
          })}
        </AnimatePresence>

        {/* Typing indicator */}
        {typing && (
          <motion.div
            initial={{ opacity: 0, y: 5 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="flex justify-start"
          >
            <div className="px-4 py-2.5 rounded-2xl rounded-bl-md bg-[var(--color-surface)] border border-[var(--color-border)]">
              <div className="flex gap-1">
                {[0, 1, 2].map((i) => (
                  <motion.div
                    key={i}
                    animate={{ y: [0, -4, 0] }}
                    transition={{
                      repeat: Infinity,
                      duration: 0.6,
                      delay: i * 0.15,
                    }}
                    className="w-1.5 h-1.5 rounded-full bg-[var(--color-text-secondary)]"
                  />
                ))}
              </div>
            </div>
          </motion.div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input bar */}
      <div className="px-4 py-3 border-t border-[var(--color-border)] bg-[var(--color-surface)]">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && handleSend()}
            placeholder="Type a message..."
            className="flex-1 px-4 py-2.5 rounded-xl border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
          />
          <Button
            onClick={handleSend}
            disabled={!input.trim()}
            className="!rounded-xl"
          >
            <Send className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
