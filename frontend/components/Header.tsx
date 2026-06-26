"use client";

import React, { useState, useEffect } from "react";
import { 
  Bell, 
  Satellite, 
  ChevronDown, 
  Clock, 
  Wifi, 
  Search 
} from "lucide-react";

interface HeaderProps {
  activeTab: string;
  selectedSatellite: string;
  setSelectedSatellite: (sat: string) => void;
  satellites: Array<{ id: string; name: string; status: string }>;
}

export default function Header({ 
  activeTab, 
  selectedSatellite, 
  setSelectedSatellite,
  satellites 
}: HeaderProps) {
  const [utcTime, setUtcTime] = useState("");
  const [showNotifications, setShowNotifications] = useState(false);

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setUtcTime(now.toISOString().replace("T", " ").substring(0, 19) + " UTC");
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  const activeSatelliteName = satellites.find(s => s.id === selectedSatellite)?.name || "Select Satellite";

  return (
    <header className="glass-panel border-b border-[#27272a] h-16 flex items-center justify-between px-6 sticky top-0 z-40">
      {/* Title Area */}
      <div className="flex items-center gap-4">
        <h2 className="text-lg font-bold text-white capitalize tracking-wide">
          {activeTab === "dashboard" ? "System Telemetry" : activeTab}
        </h2>
        <div className="h-4 w-px bg-zinc-800" />
        
        {/* Satellite Dropdown */}
        <div className="relative group">
          <button className="flex items-center gap-2 bg-[#18181b] border border-[#27272a] hover:border-zinc-700 px-3 py-1.5 rounded-lg text-xs text-zinc-300 transition-colors">
            <Satellite className="h-3.5 w-3.5 text-indigo-400" />
            <span className="font-mono">{activeSatelliteName}</span>
            <ChevronDown className="h-3 w-3 text-zinc-500" />
          </button>
          
          <div className="absolute top-full left-0 mt-1 w-48 bg-[#18181b] border border-[#27272a] rounded-lg shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-150 z-50">
            {satellites.map((sat) => (
              <button
                key={sat.id}
                onClick={() => setSelectedSatellite(sat.id)}
                className={`w-full text-left px-3 py-2 text-xs font-mono transition-colors hover:bg-zinc-800 flex items-center justify-between ${
                  selectedSatellite === sat.id ? "text-indigo-400 bg-zinc-800/40" : "text-zinc-400"
                }`}
              >
                <span>{sat.name}</span>
                <span className={`h-1.5 w-1.5 rounded-full ${
                  sat.status === "nominal" ? "bg-emerald-500" : sat.status === "warning" ? "bg-amber-500" : "bg-rose-500"
                }`} />
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Right Side Widgets */}
      <div className="flex items-center gap-6">
        {/* Search */}
        <div className="relative max-w-xs hidden md:block">
          <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-zinc-500" />
          <input
            type="text"
            placeholder="Search parameters..."
            className="bg-[#18181b] border border-[#27272a] text-xs text-zinc-300 pl-8 pr-3 py-2 rounded-lg focus:outline-none focus:border-zinc-700 w-48 font-mono placeholder-zinc-600"
          />
        </div>

        {/* UTC Clock */}
        <div className="flex items-center gap-2 text-xs font-mono text-zinc-400 bg-[#18181b] border border-[#27272a] px-3 py-1.5 rounded-lg">
          <Clock className="h-3.5 w-3.5 text-zinc-500" />
          <span>{utcTime}</span>
        </div>

        {/* Network Status */}
        <div className="flex items-center gap-2 text-xs font-mono text-emerald-400 bg-emerald-500/5 border border-emerald-500/10 px-3 py-1.5 rounded-lg">
          <Wifi className="h-3.5 w-3.5 animate-pulse" />
          <span className="hidden sm:inline">UPLINK OK</span>
        </div>

        {/* Notifications */}
        <div className="relative">
          <button 
            onClick={() => setShowNotifications(!showNotifications)}
            className="p-2 bg-[#18181b] border border-[#27272a] hover:border-zinc-700 rounded-lg text-zinc-400 hover:text-zinc-200 transition-colors relative"
          >
            <Bell className="h-4 w-4" />
            <span className="absolute top-1 right-1 h-2 w-2 bg-indigo-500 rounded-full animate-ping" />
            <span className="absolute top-1 right-1 h-2 w-2 bg-indigo-500 rounded-full" />
          </button>
          
          {showNotifications && (
            <div className="absolute right-0 mt-2 w-80 bg-[#18181b] border border-[#27272a] rounded-lg shadow-xl p-3 z-50">
              <h3 className="text-xs font-bold text-zinc-300 font-mono border-b border-[#27272a] pb-2 mb-2">SYSTEM ALERTS</h3>
              <div className="space-y-2 max-h-60 overflow-y-auto">
                <div className="p-2 bg-zinc-800/30 rounded border-l-2 border-amber-500 text-[11px] font-mono text-zinc-400">
                  <span className="text-amber-400 font-bold">[WARN]</span> Orbit decay deviation detected (ISS).
                </div>
                <div className="p-2 bg-zinc-800/30 rounded border-l-2 border-emerald-500 text-[11px] font-mono text-zinc-400">
                  <span className="text-emerald-400 font-bold">[INFO]</span> Solar array alignment optimized.
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
