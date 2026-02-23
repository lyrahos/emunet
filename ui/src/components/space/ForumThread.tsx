import { motion } from "framer-motion";
import { MessageSquare, Sprout } from "lucide-react";
import { formatRelativeTime, formatSeeds } from "@/lib/format";
import type { ContentItem } from "./LayoutRenderer";

interface ForumThreadProps {
  items: ContentItem[];
  onItemClick: (hash: string) => void;
}

export default function ForumThread({ items, onItemClick }: ForumThreadProps) {
  return (
    <div className="space-y-2">
      {items.map((item, index) => (
        <motion.button
          key={item.hash}
          initial={{ opacity: 0, x: -10 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: index * 0.03 }}
          whileHover={{ x: 2 }}
          onClick={() => onItemClick(item.hash)}
          className="w-full text-left p-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] hover:shadow-sm transition-shadow"
        >
          <div className="flex items-start gap-3">
            <div className="w-9 h-9 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center flex-shrink-0 mt-0.5">
              <MessageSquare className="h-4 w-4 text-[var(--color-accent)]" />
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <h4 className="text-sm font-semibold text-[var(--color-text)] truncate">
                  {item.title}
                </h4>
                {item.price > 0 && (
                  <span className="flex items-center gap-1 text-xs text-[var(--color-accent)] flex-shrink-0">
                    <Sprout className="h-3 w-3" />
                    {formatSeeds(item.price)}
                  </span>
                )}
              </div>
              {item.description && (
                <p className="text-sm text-[var(--color-text-secondary)] mt-0.5 line-clamp-2">
                  {item.description}
                </p>
              )}
              <div className="flex items-center gap-3 mt-2 text-xs text-[var(--color-text-secondary)]">
                <span>{item.creatorName}</span>
                <span>{formatRelativeTime(item.createdAt)}</span>
              </div>
            </div>
          </div>
        </motion.button>
      ))}
    </div>
  );
}
