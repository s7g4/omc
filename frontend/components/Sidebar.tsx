"use client";

import React from "react";
import { 
  Radio, 
  Database, 
  ShieldAlert, 
  Terminal, 
  Activity, 
  LogOut,
  FolderKanban
} from "lucide-react";

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  onLogout: () => void;
}

const Sidebar = React.memo(function Sidebar({ activeTab, setActiveTab, onLogout }: SidebarProps) {
  const menuItems = [
    { id: "dashboard", label: "Telemetry Live", icon: Activity },
    { id: "missions", label: "Missions", icon: FolderKanban },
    { id: "controls", label: "Simulator Ingress", icon: ShieldAlert },
    { id: "console", label: "Terminal Console", icon: Terminal },
    { id: "database", label: "Timescale DB", icon: Database },
  ];

  return (
    <aside className="w-64 glass-panel border-r border-[#27272a] h-screen flex flex-col justify-between p-4 sticky top-0">
      <div>
        {/* Brand / Logo */}
        <div className="flex items-center gap-3 px-2 py-4 mb-6">
          <div className="bg-indigo-600 p-2 rounded-lg text-white shadow-lg shadow-indigo-500/30 ring-1 ring-indigo-400">
            <Radio className="h-5 w-5 animate-pulse" />
          </div>
          <div>
            <h1 className="font-bold text-md tracking-wider text-white">OPEN MISSION</h1>
            <p className="text-[10px] text-zinc-500 font-mono tracking-widest uppercase">Telemetry C2</p>
          </div>
        </div>

        {/* Navigation Items */}
        <nav className="space-y-1">
          {menuItems.map((item) => {
            const Icon = item.icon;
            const isActive = activeTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => setActiveTab(item.id)}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-200 group ${
                  isActive
                    ? "bg-indigo-600/10 text-indigo-400 border-l-2 border-indigo-500 shadow-inner"
                    : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/30 border-l-2 border-transparent"
                }`}
              >
                <Icon className={`h-4 w-4 transition-transform duration-300 group-hover:scale-110 ${
                  isActive ? "text-indigo-400" : "text-zinc-500 group-hover:text-zinc-400"
                }`} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Footer / System Status */}
      <div className="border-t border-[#27272a] pt-4 space-y-3">
        <div className="bg-[#18181b]/50 p-3 rounded-lg border border-[#27272a]/40">
          <div className="flex items-center justify-between text-[11px] font-mono text-zinc-500 mb-1">
            <span>UPLINK FEED</span>
            <span className="flex items-center gap-1.5 text-emerald-400">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 pulse-green"></span>
              ONLINE
            </span>
          </div>
          <div className="text-[10px] font-mono text-zinc-600">
            ping: 14ms | frame: 60fps
          </div>
        </div>

        <button 
          onClick={onLogout}
          className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-zinc-500 hover:text-rose-400 hover:bg-rose-500/5 transition-all duration-200 cursor-pointer"
        >
          <LogOut className="h-4 w-4" />
          <span>Exit Session</span>
        </button>
      </div>
    </aside>
  );
})

export default Sidebar;
