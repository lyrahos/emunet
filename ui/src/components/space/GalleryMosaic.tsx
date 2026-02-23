import { motion } from "framer-motion";
import { Image as ImageIcon, Sprout } from "lucide-react";
import { formatSeeds } from "@/lib/format";
import type { ContentItem } from "./LayoutRenderer";

interface GalleryMosaicProps {
  items: ContentItem[];
  onItemClick: (hash: string) => void;
}

export default function GalleryMosaic({
  items,
  onItemClick,
}: GalleryMosaicProps) {
  return (
    <div className="columns-2 md:columns-3 lg:columns-4 gap-3 space-y-3">
      {items.map((item, index) => (
        <motion.div
          key={item.hash}
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: index * 0.03 }}
          whileHover={{ scale: 1.02 }}
          onClick={() => onItemClick(item.hash)}
          className="break-inside-avoid rounded-xl overflow-hidden cursor-pointer relative group"
        >
          {item.thumbnail ? (
            <img
              src={item.thumbnail}
              alt={item.title}
              className="w-full object-cover"
            />
          ) : (
            <div className="aspect-square bg-[var(--color-border)]/30 flex items-center justify-center">
              <ImageIcon className="h-10 w-10 text-[var(--color-text-secondary)]/30" />
            </div>
          )}

          {/* Overlay on hover */}
          <div className="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-3">
            <p className="text-sm font-semibold text-white truncate">
              {item.title}
            </p>
            <div className="flex items-center justify-between mt-1">
              <p className="text-xs text-white/70">{item.creatorName}</p>
              {item.price > 0 && !item.owned && (
                <span className="flex items-center gap-1 text-xs text-white font-semibold">
                  <Sprout className="h-3 w-3" />
                  {formatSeeds(item.price)}
                </span>
              )}
            </div>
          </div>
        </motion.div>
      ))}
    </div>
  );
}
