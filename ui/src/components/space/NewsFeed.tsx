import { motion } from "framer-motion";
import { Sprout, CheckCircle2 } from "lucide-react";
import Badge from "@/components/ui/Badge";
import { formatRelativeTime, formatSeeds } from "@/lib/format";
import type { ContentItem } from "./LayoutRenderer";

interface NewsFeedProps {
  items: ContentItem[];
  onItemClick: (hash: string) => void;
}

export default function NewsFeed({ items, onItemClick }: NewsFeedProps) {
  return (
    <div className="space-y-4 max-w-2xl mx-auto">
      {items.map((item, index) => (
        <motion.article
          key={item.hash}
          initial={{ opacity: 0, y: 15 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: index * 0.05 }}
          onClick={() => onItemClick(item.hash)}
          className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] overflow-hidden cursor-pointer hover:shadow-md transition-shadow"
        >
          {/* Header */}
          <div className="flex items-center gap-3 px-4 pt-4">
            <div className="w-9 h-9 rounded-full bg-[var(--color-accent)]/10 flex items-center justify-center">
              <span className="text-sm font-semibold text-[var(--color-accent)]">
                {item.creatorName.charAt(0).toUpperCase()}
              </span>
            </div>
            <div>
              <p className="text-sm font-semibold text-[var(--color-text)]">
                {item.creatorName}
              </p>
              <p className="text-xs text-[var(--color-text-secondary)]">
                {formatRelativeTime(item.createdAt)}
              </p>
            </div>
          </div>

          {/* Content */}
          <div className="px-4 py-3">
            <h3 className="text-base font-semibold text-[var(--color-text)]">
              {item.title}
            </h3>
            {item.description && (
              <p className="text-sm text-[var(--color-text-secondary)] mt-1 line-clamp-3">
                {item.description}
              </p>
            )}
          </div>

          {/* Thumbnail */}
          {item.thumbnail && (
            <div className="aspect-video bg-[var(--color-border)]/30">
              <img
                src={item.thumbnail}
                alt={item.title}
                className="w-full h-full object-cover"
              />
            </div>
          )}

          {/* Footer: price/status */}
          <div className="px-4 py-3 border-t border-[var(--color-border)] flex items-center justify-between">
            {item.owned ? (
              <Badge variant="yours">
                <CheckCircle2 className="h-3 w-3 mr-1" />
                Yours
              </Badge>
            ) : item.price === 0 ? (
              <Badge variant="free">Free</Badge>
            ) : (
              <span className="flex items-center gap-1 text-sm font-semibold text-[var(--color-accent)]">
                <Sprout className="h-4 w-4" />
                {formatSeeds(item.price)}
              </span>
            )}
          </div>
        </motion.article>
      ))}
    </div>
  );
}
