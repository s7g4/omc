"use client";

import React from "react";
import { 
  ShieldCheck, 
  Flame, 
  Zap, 
  Compass, 
  Gauge, 
  Map 
} from "lucide-react";

interface TelemetryData {
  battery_level: number;
  battery_temp: number;
  solar_power: number;
  velocity: number;
  altitude: number;
  latitude: number;
  longitude: number;
}

interface TelemetryGridProps {
  data: TelemetryData | null;
}

export default function TelemetryGrid({ data }: TelemetryGridProps) {
  // Safe fallback if data is not loaded yet
  const stats = {
    altitude: data?.altitude ?? 421.5,
    velocity: data?.velocity ?? 7.66,
    battery: data?.battery_level ?? 98.4,
    temp: data?.battery_temp ?? 24.2,
    solar: data?.solar_power ?? 120.5,
    latitude: data?.latitude ?? 51.64,
    longitude: data?.longitude ?? -0.12,
  };

  const cards = [
    {
      title: "Orbital Altitude",
      value: `${stats.altitude.toFixed(2)} km`,
      status: "NOMINAL",
      statusColor: "text-emerald-400",
      indicatorColor: "bg-emerald-500",
      pulseClass: "pulse-green",
      desc: "Low Earth Orbit (LEO) stable",
      icon: Compass,
      color: "from-indigo-500/10 to-transparent",
      borderColor: "group-hover:border-indigo-500/40",
      accent: "text-indigo-400"
    },
    {
      title: "Orbital Velocity",
      value: `${stats.velocity.toFixed(4)} km/s`,
      status: "NOMINAL",
      statusColor: "text-emerald-400",
      indicatorColor: "bg-emerald-500",
      pulseClass: "pulse-green",
      desc: "Mach 22.5 relative velocity",
      icon: Gauge,
      color: "from-violet-500/10 to-transparent",
      borderColor: "group-hover:border-violet-500/40",
      accent: "text-violet-400"
    },
    {
      title: "Power System (Battery)",
      value: `${stats.battery.toFixed(1)}%`,
      status: stats.battery < 20 ? "CRITICAL" : stats.battery < 50 ? "WARNING" : "NOMINAL",
      statusColor: stats.battery < 20 ? "text-rose-400" : stats.battery < 50 ? "text-amber-400" : "text-emerald-400",
      indicatorColor: stats.battery < 20 ? "bg-rose-500" : stats.battery < 50 ? "bg-amber-500" : "bg-emerald-500",
      pulseClass: stats.battery < 20 ? "pulse-rose" : stats.battery < 50 ? "pulse-amber" : "pulse-green",
      desc: `Solar generation: ${stats.solar.toFixed(1)} W`,
      icon: Zap,
      color: "from-amber-500/10 to-transparent",
      borderColor: "group-hover:border-amber-500/40",
      accent: "text-amber-400"
    },
    {
      title: "Core / Battery Temp",
      value: `${stats.temp.toFixed(1)} °C`,
      status: stats.temp > 45 ? "CRITICAL" : stats.temp > 35 ? "WARNING" : "NOMINAL",
      statusColor: stats.temp > 45 ? "text-rose-400" : stats.temp > 35 ? "text-amber-400" : "text-emerald-400",
      indicatorColor: stats.temp > 45 ? "bg-rose-500" : stats.temp > 35 ? "bg-amber-500" : "bg-emerald-500",
      pulseClass: stats.temp > 45 ? "pulse-rose" : stats.temp > 35 ? "pulse-amber" : "pulse-green",
      desc: "Thermal control active",
      icon: Flame,
      color: "from-rose-500/10 to-transparent",
      borderColor: "group-hover:border-rose-500/40",
      accent: "text-rose-400"
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {cards.map((card, idx) => {
        const Icon = card.icon;
        return (
          <div 
            key={idx} 
            className={`group relative glass-panel rounded-xl p-5 overflow-hidden transition-all duration-300 hover:translate-y-[-2px] border-[#27272a]/50 hover:shadow-lg hover:shadow-indigo-950/10 ${card.borderColor}`}
          >
            {/* Hover Gradient Overlay */}
            <div className={`absolute inset-0 bg-gradient-to-br ${card.color} opacity-30 pointer-events-none transition-opacity duration-300`} />
            
            <div className="relative flex justify-between items-start mb-4">
              <div>
                <span className="text-[10px] font-mono tracking-widest text-zinc-500 uppercase block mb-1">
                  {card.title}
                </span>
                <span className="text-2xl font-bold font-mono tracking-tight text-white block">
                  {card.value}
                </span>
              </div>
              <div className={`p-2 rounded-lg bg-zinc-800/40 border border-[#27272a] ${card.accent}`}>
                <Icon className="h-4.5 w-4.5" />
              </div>
            </div>

            <div className="relative flex items-center justify-between mt-6 pt-3 border-t border-[#27272a]/30">
              <span className="text-[10px] font-mono text-zinc-400">
                {card.desc}
              </span>
              <span className={`flex items-center gap-1.5 text-[9px] font-mono font-bold tracking-wider px-2 py-0.5 rounded bg-zinc-800/40 border border-[#27272a] ${card.statusColor}`}>
                <span className={`h-1.5 w-1.5 rounded-full ${card.indicatorColor} ${card.pulseClass}`}></span>
                {card.status}
              </span>
            </div>
          </div>
        );
      })}

      {/* Lat/Lon Card - Spans full width on small but fits nicely */}
      <div className="col-span-1 md:col-span-2 lg:col-span-4 glass-panel border-[#27272a]/50 rounded-xl p-4 flex flex-col sm:flex-row items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400">
            <Map className="h-5 w-5" />
          </div>
          <div>
            <span className="text-[10px] font-mono tracking-widest text-zinc-500 uppercase block">ORBITAL COORDINATES</span>
            <span className="text-sm font-semibold text-zinc-200 block font-mono">
              Latitude: <span className="text-white">{stats.latitude.toFixed(6)}°</span> | Longitude: <span className="text-white">{stats.longitude.toFixed(6)}°</span>
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2 font-mono text-[11px] text-zinc-400 bg-[#18181b] border border-[#27272a] px-3 py-1.5 rounded-lg">
          <ShieldCheck className="h-4 w-4 text-emerald-400" />
          <span>GEO-LOCATION RESOLVED (LEO STATIONARY ALIGNMENT)</span>
        </div>
      </div>
    </div>
  );
}
