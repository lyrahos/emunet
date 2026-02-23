import { useEffect, useRef } from "react";
import { eventBus } from "@/lib/event-bus";

/**
 * Hook to subscribe to daemon events. Automatically unsubscribes on cleanup.
 *
 * @param topic - The event topic to subscribe to
 * @param handler - Callback invoked with the event payload
 * @param enabled - Whether the subscription is active (default: true)
 */
export function useEvents(
  topic: string,
  handler: (payload: unknown) => void,
  enabled = true,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!enabled) return;

    let unsubscribe: (() => void) | null = null;

    eventBus.init().then(() => {
      eventBus
        .subscribe(topic, (payload) => handlerRef.current(payload))
        .then((unsub) => {
          unsubscribe = unsub;
        });
    });

    return () => {
      if (unsubscribe) unsubscribe();
    };
  }, [topic, enabled]);
}
