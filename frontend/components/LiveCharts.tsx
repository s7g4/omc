"use client";

import React, { useState, useEffect } from "react";
import { 
  AreaChart, 
  Area, 
  LineChart, 
  Line, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer 
} from "recharts";

interface HistoricalData {
  time: string;
  altitude: number;
  velocity: number;
  temp: number;
  solar: number;
}

interface LiveChartsProps {
  history: HistoricalData[];
}

export default function LiveCharts({ history }: LiveChartsProps) {
  const [mounted, setMounted] = useState(false);
  const [activeChartTab, setActiveChartTab] = useState<"altitude" | "velocity" | "power">("altitude");

  useEffect(() => {
    const timer = setTimeout(() => {
      setMounted(true);
    }, 0);
    return () => clearTimeout(timer);
  }, []);

  if (!mounted) {
    return (
      <div className="glass-panel border-[#27272a]/50 rounded-xl p-6 h-96 flex items-center justify-center">
        <span className="text-sm font-mono text-zinc-500 animate-pulse">BOOTING CHART RENDERING ENGINE...</span>
      </div>
    );
  }

  // Formatting timestamp for X-Axis (showing seconds)
  const formatXAxis = (tickItem: string) => {
    try {
      const parts = tickItem.split(":");
      return parts.length >= 3 ? `${parts[1]}:${parts[2]}` : tickItem;
    } catch {
      return tickItem;
    }
  };

  const tabs = [
    { id: "altitude", label: "Altitude Path" },
    { id: "velocity", label: "Velocity Profile" },
    { id: "power", label: "Power & Thermal" },
  ];

  return (
    <div className="glass-panel border-[#27272a]/50 rounded-xl p-6 flex flex-col h-[400px]">
      {/* Header and Tabs */}
      <div className="flex items-center justify-between border-b border-[#27272a]/30 pb-4 mb-4">
        <div>
          <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider">Historical Analytics</h3>
          <p className="text-[10px] text-zinc-500 font-mono">Live telemetry stream charting</p>
        </div>

        <div className="flex bg-[#18181b] border border-[#27272a] rounded-lg p-0.5">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveChartTab(tab.id as "altitude" | "velocity" | "power")}
              className={`px-3 py-1.5 rounded-md text-[11px] font-mono font-medium transition-all duration-200 ${
                activeChartTab === tab.id
                  ? "bg-indigo-600 text-white shadow-md shadow-indigo-500/25"
                  : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Chart container */}
      <div className="flex-1 w-full min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          {activeChartTab === "altitude" ? (
            <AreaChart data={history} margin={{ top: 10, right: 5, left: -20, bottom: 0 }}>
              <defs>
                <linearGradient id="colorAlt" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#6366f1" stopOpacity={0.2}/>
                  <stop offset="95%" stopColor="#6366f1" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="rgba(39, 39, 42, 0.3)" />
              <XAxis 
                dataKey="time" 
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
                tickFormatter={formatXAxis}
              />
              <YAxis 
                domain={["auto", "auto"]} 
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
              />
              <Tooltip 
                contentStyle={{ 
                  backgroundColor: "rgba(18, 18, 24, 0.9)", 
                  borderColor: "rgba(99, 102, 241, 0.4)", 
                  color: "#ffffff",
                  fontSize: "11px",
                  fontFamily: "monospace",
                  borderRadius: "8px"
                }} 
              />
              <Area 
                type="monotone" 
                dataKey="altitude" 
                name="Altitude (km)"
                stroke="#6366f1" 
                strokeWidth={2}
                fillOpacity={1} 
                fill="url(#colorAlt)" 
              />
            </AreaChart>
          ) : activeChartTab === "velocity" ? (
            <AreaChart data={history} margin={{ top: 10, right: 5, left: -20, bottom: 0 }}>
              <defs>
                <linearGradient id="colorVel" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.2}/>
                  <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="rgba(39, 39, 42, 0.3)" />
              <XAxis 
                dataKey="time" 
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
                tickFormatter={formatXAxis}
              />
              <YAxis 
                domain={["auto", "auto"]} 
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
              />
              <Tooltip 
                contentStyle={{ 
                  backgroundColor: "rgba(18, 18, 24, 0.9)", 
                  borderColor: "rgba(139, 92, 246, 0.4)", 
                  color: "#ffffff",
                  fontSize: "11px",
                  fontFamily: "monospace",
                  borderRadius: "8px"
                }} 
              />
              <Area 
                type="monotone" 
                dataKey="velocity" 
                name="Velocity (km/s)"
                stroke="#8b5cf6" 
                strokeWidth={2}
                fillOpacity={1} 
                fill="url(#colorVel)" 
              />
            </AreaChart>
          ) : (
            <LineChart data={history} margin={{ top: 10, right: 5, left: -20, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="rgba(39, 39, 42, 0.3)" />
              <XAxis 
                dataKey="time" 
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
                tickFormatter={formatXAxis}
              />
              <YAxis 
                yAxisId="left"
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
              />
              <YAxis 
                yAxisId="right"
                orientation="right"
                tick={{ fill: "#71717a", fontSize: 10, fontFamily: "monospace" }} 
                stroke="rgba(39, 39, 42, 0.4)" 
              />
              <Tooltip 
                contentStyle={{ 
                  backgroundColor: "rgba(18, 18, 24, 0.9)", 
                  borderColor: "rgba(63, 63, 70, 0.4)", 
                  color: "#ffffff",
                  fontSize: "11px",
                  fontFamily: "monospace",
                  borderRadius: "8px"
                }} 
              />
              <Line 
                yAxisId="left"
                type="monotone" 
                dataKey="solar" 
                name="Solar Output (W)"
                stroke="#f59e0b" 
                strokeWidth={2}
                dot={false}
              />
              <Line 
                yAxisId="right"
                type="monotone" 
                dataKey="temp" 
                name="Temp (°C)"
                stroke="#f43f5e" 
                strokeWidth={2}
                dot={false}
              />
            </LineChart>
          )}
        </ResponsiveContainer>
      </div>
    </div>
  );
}
