import { useState, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { motion, AnimatePresence } from "framer-motion";
import { Search, Plus, UserPlus, Inbox } from "lucide-react";
import Button from "@/components/ui/Button";
import SpaceCard from "@/components/space/SpaceCard";
import { useSpaces } from "@/hooks/useSpaces";

export default function Home() {
  const navigate = useNavigate();
  const { spaces } = useSpaces();
  const [query, setQuery] = useState("");

  const showSearch = spaces.length >= 8;

  const filtered = useMemo(() => {
    if (!query.trim()) return spaces;
    const q = query.toLowerCase();
    return spaces.filter((s) => s.name.toLowerCase().includes(q));
  }, [spaces, query]);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="flex items-center justify-between px-6 pt-6 pb-4">
        <h1 className="text-xl font-bold text-[var(--color-text)]">Spaces</h1>
        <Button size="sm" onClick={() => navigate("/space/new")}>
          <Plus className="h-4 w-4" />
          New Space
        </Button>
      </header>

      {/* Search (shown at 8+ spaces) */}
      {showSearch && (
        <div className="px-6 pb-4">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-[var(--color-text-secondary)]" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search Spaces..."
              className="w-full pl-10 pr-4 py-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
            />
          </div>
        </div>
      )}

      {/* Space list */}
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {filtered.length > 0 ? (
          <motion.div layout className="grid gap-3">
            <AnimatePresence mode="popLayout">
              {filtered.map((space) => (
                <motion.div
                  key={space.groupId}
                  layout
                  initial={{ opacity: 0, scale: 0.95 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                >
                  <SpaceCard space={space} />
                </motion.div>
              ))}
            </AnimatePresence>
          </motion.div>
        ) : spaces.length === 0 ? (
          // Empty state
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="flex flex-col items-center justify-center h-full text-center space-y-6"
          >
            <div className="w-20 h-20 rounded-2xl bg-[var(--color-accent)]/10 flex items-center justify-center">
              <Inbox className="h-10 w-10 text-[var(--color-accent)]" />
            </div>
            <div className="space-y-2">
              <h2 className="text-lg font-semibold text-[var(--color-text)]">
                No Spaces Yet
              </h2>
              <p className="text-sm text-[var(--color-text-secondary)] max-w-xs">
                Create your own Space to share content, or ask someone for an
                invite to join theirs.
              </p>
            </div>
            <div className="flex gap-3">
              <Button onClick={() => navigate("/space/new")}>
                <Plus className="h-4 w-4" />
                Create Space
              </Button>
              <Button variant="secondary">
                <UserPlus className="h-4 w-4" />
                Join with Invite
              </Button>
            </div>
          </motion.div>
        ) : (
          // No search results
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <Search className="h-8 w-8 text-[var(--color-text-secondary)]/40 mb-3" />
            <p className="text-sm text-[var(--color-text-secondary)]">
              No Spaces match "{query}"
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
