"use client";

import React, { useState, useEffect, useRef } from "react";
import { 
  Terminal, 
  Trash2 
} from "lucide-react";

export interface LogEvent {
  id: string;
  timestamp: string;
  type: "info" | "warning" | "error";
  subsystem: string;
  message: string;
}

interface EventsLogProps {
  events: LogEvent[];
  clearEvents: () => void;
}

export default function EventsLog({ events, clearEvents }: EventsLogProps) {
  const [filter, setFilter] = useState<"all" | "info" | "warning" | "error">("all");
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto scroll terminal to the bottom when new events arrive
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [events]);

  const filteredEvents = events.filter((e) => {
    if (filter === "all") return true;
    return e.type === filter;
  });

  return (
    <div className="glass-panel border-[#27272a]/50 rounded-xl p-5 flex flex-col h-[400px]">
      {/* Terminal Title Header */}
      <div className="flex items-center justify-between border-b border-[#27272a]/30 pb-3 mb-3">
        <div className="flex items-center gap-2">
          <Terminal className="h-4.5 w-4.5 text-indigo-400" />
          <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider">Event Stream Console</h3>
        </div>

        <div className="flex items-center gap-3">
          {/* Filter */}
          <div className="flex bg-[#18181b] border border-[#27272a] rounded-lg p-0.5">
            {(["all", "info", "warning", "error"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setFilter(t)}
                className={`px-2 py-1 rounded text-[10px] font-mono capitalize transition-all ${
                  filter === t
                    ? "bg-zinc-800 text-white font-semibold"
                    : "text-zinc-500 hover:text-zinc-300"
                }`}
              >
                {t}
              </button>
            ))}
          </div>

          {/* Clear button */}
          <button
            onClick={clearEvents}
            className="p-1.5 bg-[#18181b] border border-[#27272a] hover:border-zinc-700 rounded-lg text-zinc-500 hover:text-rose-400 transition-colors"
            title="Clear logs"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {/* Terminal logs list */}
      <div 
        ref={containerRef}
        className="flex-1 overflow-y-auto bg-[#09090b]/80 border border-[#27272a]/50 rounded-lg p-4 font-mono text-[11px] space-y-2 select-text"
      >
        {filteredEvents.length === 0 ? (
          <div className="h-full flex items-center justify-center text-zinc-600 italic">
            &gt; No events recorded in stream buffer
          </div>
        ) : (
          filteredEvents.map((e) => {
            let typeColor = "text-emerald-400";
            let typeBg = "bg-emerald-500/10 border-emerald-500/20";
            if (e.type === "warning") {
              typeColor = "text-amber-400";
              typeBg = "bg-amber-500/10 border-amber-500/20";
            } else if (e.type === "error") {
              typeColor = "text-rose-400";
              typeBg = "bg-rose-500/10 border-rose-500/20";
            }

            return (
              <div 
                key={e.id} 
                className="flex items-start gap-2 py-1 px-2 rounded hover:bg-zinc-900/50 border border-transparent hover:border-[#27272a]/30 transition-all"
              >
                <span className="text-zinc-600 shrink-0">{e.timestamp}</span>
                <span className={`px-1.5 py-0.5 rounded border text-[9px] font-bold ${typeBg} ${typeColor} shrink-0`}>
                  {e.subsystem}
                </span>
                <span className="text-zinc-300 flex-1 break-all">{e.message}</span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
