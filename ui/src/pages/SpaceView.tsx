import { useState, useEffect, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import {
  Search,
  Settings,
  Users,
  BarChart3,
  ArrowLeft,
} from "lucide-react";
import Badge from "@/components/ui/Badge";
import LayoutRenderer, {
  type ContentItem,
} from "@/components/space/LayoutRenderer";
import { useAppStore, type Space } from "@/lib/store";
import { rpcCall } from "@/lib/rpc-client";

export default function SpaceView() {
  const { groupId } = useParams<{ groupId: string }>();
  const navigate = useNavigate();
  const spaces = useAppStore((s) => s.spaces);
  const setActiveGroupId = useAppStore((s) => s.setActiveGroupId);

  const [space, setSpace] = useState<Space | null>(null);
  const [items, setItems] = useState<ContentItem[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (groupId) {
      setActiveGroupId(groupId);
      const found = spaces.find((s) => s.groupId === groupId);
      if (found) setSpace(found);
    }
    return () => setActiveGroupId(null);
  }, [groupId, spaces, setActiveGroupId]);

  const fetchContent = useCallback(async () => {
    if (!groupId) return;
    setLoading(true);
    try {
      const res = await rpcCall<ContentItem[]>("get_group_content", {
        group_id: groupId,
      });
      setItems(res);
    } catch {
      // Content may not be available yet
    } finally {
      setLoading(false);
    }
  }, [groupId]);

  useEffect(() => {
    fetchContent();
  }, [fetchContent]);

  const filteredItems = query.trim()
    ? items.filter((i) =>
        i.title.toLowerCase().includes(query.toLowerCase()),
      )
    : items;

  const handleItemClick = (hash: string) => {
    const item = items.find((i) => i.hash === hash);
    if (item?.owned || item?.price === 0) {
      // Already owned or free - open viewer
      // For now, navigate to checkout which handles both cases
      navigate(`/checkout/${hash}`);
    } else {
      navigate(`/checkout/${hash}`);
    }
  };

  if (!space) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-[var(--color-text-secondary)]">Loading Space...</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="border-b border-[var(--color-border)] px-6 py-4">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate("/")}
            className="p-1.5 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
          <div className="w-10 h-10 rounded-xl bg-[var(--color-accent)]/10 flex items-center justify-center">
            {space.icon ? (
              <span className="text-lg">{space.icon}</span>
            ) : (
              <span className="text-sm font-semibold text-[var(--color-accent)]">
                {space.name.charAt(0).toUpperCase()}
              </span>
            )}
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h1 className="text-lg font-bold text-[var(--color-text)] truncate">
                {space.name}
              </h1>
              <Badge variant={space.role} />
            </div>
            <p className="text-xs text-[var(--color-text-secondary)]">
              {space.memberCount} members
            </p>
          </div>

          {/* Host actions */}
          {space.role === "host" && (
            <div className="flex gap-1.5">
              <button
                onClick={() => navigate(`/dashboard/${groupId}`)}
                className="p-2 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
                title="Dashboard"
              >
                <BarChart3 className="h-5 w-5" />
              </button>
              <button
                onClick={() => navigate(`/space/${groupId}/people`)}
                className="p-2 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
                title="Members"
              >
                <Users className="h-5 w-5" />
              </button>
              <button
                onClick={() => navigate(`/space/${groupId}/settings`)}
                className="p-2 rounded-lg text-[var(--color-text-secondary)] hover:text-[var(--color-text)] hover:bg-[var(--color-border)]/30 transition-colors"
                title="Settings"
              >
                <Settings className="h-5 w-5" />
              </button>
            </div>
          )}
        </div>

        {/* Search */}
        <div className="mt-3 relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-[var(--color-text-secondary)]" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search content..."
            className="w-full pl-10 pr-4 py-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] text-[var(--color-text)] placeholder:text-[var(--color-text-secondary)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]/50 text-sm"
          />
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {loading ? (
          <div className="flex items-center justify-center py-16">
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
              className="w-6 h-6 border-2 border-[var(--color-accent)] border-t-transparent rounded-full"
            />
          </div>
        ) : filteredItems.length > 0 ? (
          <LayoutRenderer
            template={space.template}
            items={filteredItems}
            onItemClick={handleItemClick}
          />
        ) : (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <p className="text-sm text-[var(--color-text-secondary)]">
              {query ? `No content matches "${query}"` : "No content yet"}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
