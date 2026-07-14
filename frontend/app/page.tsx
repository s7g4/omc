"use client";

import React, { useState, useEffect, useRef } from "react";
import Sidebar from "@/components/Sidebar";
import Header from "@/components/Header";
import TelemetryGrid from "@/components/TelemetryGrid";
import LiveCharts from "@/components/LiveCharts";
import EventsLog, { LogEvent } from "@/components/EventsLog";
import SimControl from "@/components/SimControl";
import MissionsOverview, { Mission } from "@/components/MissionsOverview";
import AuthScreen from "@/components/AuthScreen";
import { Database, Cpu, Network, HelpCircle, Terminal, Loader2 } from "lucide-react";
import { API_URL, WS_URL } from "@/lib/config";

// Static definitions of Satellites
const SATELLITES = [
  { id: "f81d4fae-7dec-11d0-a765-00a0c91e6bf6", name: "ISS (Zarya)", status: "nominal" },
  { id: "38b4eb99-1a76-4d05-992e-9d22ffce9328", name: "Hubble Space Telescope", status: "nominal" },
  { id: "a57e3f89-8d7b-4a5f-ba0a-7e3f982cfda5", name: "Sentinel-6 Earth Observer", status: "nominal" },
];

interface TelemetryPayloadData {
  time: string;
  altitude: number;
  velocity: number;
  temp: number;
  solar: number;
  battery_level: number;
  battery_temp: number;
  solar_power: number;
  latitude: number;
  longitude: number;
}

interface BackendMission {
  id: string;
  name: string;
  description: string | null;
  status: string;
  start_date: string;
}

interface HistoricalAggregateItem {
  bucket_time: string;
  avg_altitude: number | null;
  avg_velocity: number | null;
  avg_battery_temp: number | null;
  avg_solar_power: number | null;
  avg_battery_level: number | null;
  avg_latitude: number | null;
  avg_longitude: number | null;
}

interface SimulatorPayload {
  satellite_id: string;
  battery_level: number;
  battery_temp: number;
  solar_power: number;
  velocity: number;
  altitude: number;
  latitude: number;
  longitude: number;
}

export default function Home() {
  // Session Authentication state
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [token, setToken] = useState("");
  const [username, setUsername] = useState("");
  const [loadingSession, setLoadingSession] = useState(true);

  // Tab and view state
  const [activeTab, setActiveTab] = useState("dashboard");
  const [selectedSatellite, setSelectedSatellite] = useState(SATELLITES[0].id);
  const [isWebSocketConnected, setIsWebSocketConnected] = useState(false);

  // Live Telemetry States
  const [telemetry, setTelemetry] = useState<TelemetryPayloadData | null>(null);
  const [history, setHistory] = useState<TelemetryPayloadData[]>([]);
  const [events, setEvents] = useState<LogEvent[]>([]);

  // Simulation State
  const [isSimulating] = useState(true);
  const [activeFault, setActiveFault] = useState<string | null>(null);

  // Missions State
  const [missions, setMissions] = useState<Mission[]>([
    {
      id: "m-1",
      name: "Orbital Habitat Thermal Survey",
      description: "Monitor ISS radiator arrays and battery core temperature gradients during solar transit orbits.",
      status: "active",
      satellite_id: SATELLITES[0].id,
      start_date: "2026-06-20",
    },
    {
      id: "m-2",
      name: "Deep Field Infrared Calibration",
      description: "Perform precise gyroscopic alignment checks for the Hubble core mirror assemblies during observation cycles.",
      status: "planned",
      satellite_id: SATELLITES[1].id,
      start_date: "2026-07-02",
    },
  ]);

  // Refs for tracking physics parameters under mock mode
  const physicsStateRef = useRef({
    altitude: 418.5,
    velocity: 7.66,
    battery: 98.4,
    temp: 24.2,
    solar: 120.0,
    lat: 51.64,
    lon: -0.12,
  });

  // Add Event Helper (declared using hoisted function syntax)
  function addEvent(type: LogEvent["type"], subsystem: string, message: string) {
    const newEvent: LogEvent = {
      id: Math.random().toString(36).substring(2, 11),
      timestamp: new Date().toISOString().substring(11, 19),
      type,
      subsystem,
      message,
    };
    setEvents((prev) => [...prev, newEvent]);
  }

  // Verify session on mount
  useEffect(() => {
    const storedToken = localStorage.getItem("jwt_token");
    const storedUser = localStorage.getItem("operator_name");
    
    // Defer state updates to avoid synchronous cascading renders warning
    const timer = setTimeout(() => {
      if (storedToken && storedUser) {
        setToken(storedToken);
        setUsername(storedUser);
        setIsAuthenticated(true);
      }
      setLoadingSession(false);
    }, 0);

    return () => clearTimeout(timer);
  }, []);

  const handleAuthSuccess = (newToken: string, newUsername: string) => {
    localStorage.setItem("jwt_token", newToken);
    localStorage.setItem("operator_name", newUsername);
    setToken(newToken);
    setUsername(newUsername);
    setIsAuthenticated(true);
    addEvent("info", "SECURITY", `Operator "${newUsername}" session authorized.`);
  };

  const handleLogout = () => {
    localStorage.removeItem("jwt_token");
    localStorage.removeItem("operator_name");
    setToken("");
    setUsername("");
    setIsAuthenticated(false);
    setActiveTab("dashboard");
  };

  // WebSocket Connection
  useEffect(() => {
    if (!isAuthenticated || !token) return;

    let ws: WebSocket | null = null;
    let reconnectTimeout: ReturnType<typeof setTimeout> | undefined;

    const connectWS = () => {
      ws = new WebSocket(
        `${WS_URL}/api/v1/telemetry/ws?satellite_id=${selectedSatellite}&token=${encodeURIComponent(token)}`
      );

      ws.onopen = () => {
        setIsWebSocketConnected(true);
        addEvent("info", "COMMS", "Established live telemetry link to backend server.");
      };

      ws.onmessage = (event) => {
        try {
          const payload = JSON.parse(event.data);

          // 1. Process as Event broadcast if severity/message fields are present
          if (payload.severity && payload.message) {
            if (payload.satellite_id === selectedSatellite) {
              const mappedType = payload.severity === "error" ? "error" : payload.severity === "warning" ? "warning" : "info";
              addEvent(mappedType, "SYSTEM", payload.message);
            }
            return;
          }

          // 2. Otherwise process as standard Telemetry log
          const tele = payload as SimulatorPayload;
          if (tele.satellite_id === selectedSatellite) {
            const formattedTime = new Date().toLocaleTimeString();
            const newData = {
              time: formattedTime,
              altitude: tele.altitude,
              velocity: tele.velocity,
              temp: tele.battery_temp,
              solar: tele.solar_power,
              battery_level: tele.battery_level,
              battery_temp: tele.battery_temp,
              solar_power: tele.solar_power,
              latitude: tele.latitude,
              longitude: tele.longitude,
            };

            setTelemetry(newData);
            setHistory((prev) => [...prev.slice(-29), newData]);

            // Append warning events if any fields are anomalous
            if (tele.battery_level < 20) {
              addEvent("error", "POWER", `Critical battery reserve: ${tele.battery_level.toFixed(1)}%`);
            }
            if (tele.battery_temp > 45) {
              addEvent("error", "THERMAL", `Subsystem overheat: ${tele.battery_temp.toFixed(1)}°C`);
            }
          }
        } catch (e) {
          console.error("Failed to parse websocket message", e);
        }
      };

      ws.onerror = () => {
        setIsWebSocketConnected(false);
      };

      ws.onclose = () => {
        setIsWebSocketConnected(false);
        // Retry connection in 5s
        reconnectTimeout = setTimeout(connectWS, 5000);
      };
    };

    connectWS();

    return () => {
      if (ws) ws.close();
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
    };
  }, [selectedSatellite, isAuthenticated, token]);

  // Load missions from backend on authentication
  useEffect(() => {
    if (!isAuthenticated || !token) return;

    const fetchMissions = async () => {
      try {
        const response = await fetch(`${API_URL}/api/v1/missions`, {
          headers: {
            "Authorization": `Bearer ${token}`
          }
        });
        if (response.ok) {
          const list = await response.json();
          const mapped = list.map((m: BackendMission) => ({
            id: m.id,
            name: m.name,
            description: m.description || "",
            status: m.status,
            satellite_id: null,
            start_date: m.start_date.split("T")[0],
          }));
          setMissions(mapped);
          addEvent("info", "COMMAND", `Loaded ${mapped.length} mission profiles from database.`);
        }
      } catch (err) {
        console.error("Failed to fetch missions from database", err);
        addEvent("warning", "COMMAND", "Database connection lost. Operating in standalone local directory.");
      }
    };

    fetchMissions();
  }, [isAuthenticated, token]);

  // Load telemetry history from backend TimescaleDB
  useEffect(() => {
    if (!isAuthenticated || !token || !selectedSatellite) return;

    const fetchHistory = async () => {
      try {
        const response = await fetch(`${API_URL}/api/v1/telemetry/${selectedSatellite}/history?bucket=10&limit=30`, {
          headers: {
            "Authorization": `Bearer ${token}`
          }
        });
        if (response.ok) {
          const list = await response.json();
          // Map TimescaleDB aggregates back into chart data points
          // Reverse list because DB query returns newest first, but chart wants chronologically oldest -> newest
          const mapped = list.map((item: HistoricalAggregateItem) => {
            const timeObj = new Date(item.bucket_time);
            const formattedTime = timeObj.toLocaleTimeString();
            return {
              time: formattedTime,
              altitude: item.avg_altitude ?? 0.0,
              velocity: item.avg_velocity ?? 0.0,
              temp: item.avg_battery_temp ?? 0.0,
              solar: item.avg_solar_power ?? 0.0,
              battery_level: item.avg_battery_level ?? 0.0,
              battery_temp: item.avg_battery_temp ?? 0.0,
              solar_power: item.avg_solar_power ?? 0.0,
              latitude: item.avg_latitude ?? 0.0,
              longitude: item.avg_longitude ?? 0.0,
            };
          }).reverse();

          setHistory(mapped);
          
          if (mapped.length > 0) {
            const latest = mapped[mapped.length - 1];
            setTelemetry(latest);
            addEvent("info", "DATABASE", `Loaded ${mapped.length} historical timeseries buckets from TimescaleDB.`);
          }
        }
      } catch (err) {
        console.error("Failed to fetch telemetry history from database", err);
      }
    };

    fetchHistory();
  }, [selectedSatellite, isAuthenticated, token]);

  // Mock Telemetry Generator Loop (runs when WebSocket is disconnected)
  useEffect(() => {
    if (!isAuthenticated || isWebSocketConnected || !isSimulating) return;

    // Reset physics state when satellite changes
    const sat = SATELLITES.find(s => s.id === selectedSatellite);
    if (sat) {
      if (sat.id === SATELLITES[0].id) { // ISS
        physicsStateRef.current = { altitude: 418.5, velocity: 7.66, battery: 98.4, temp: 24.2, solar: 120.0, lat: 51.64, lon: -0.12 };
      } else if (sat.id === SATELLITES[1].id) { // Hubble
        physicsStateRef.current = { altitude: 535.2, velocity: 7.59, battery: 85.0, temp: 18.5, solar: 185.0, lat: 28.53, lon: -80.64 };
      } else { // Sentinel
        physicsStateRef.current = { altitude: 1336.0, velocity: 7.12, battery: 92.1, temp: 21.0, solar: 145.0, lat: -34.61, lon: -58.38 };
      }
    }

    const interval = setInterval(() => {
      const state = physicsStateRef.current;

      // Basic orbital tracking logic
      state.lon += 0.5;
      if (state.lon > 180) state.lon = -180;
      
      state.lat = Math.sin(state.lon * (Math.PI / 180)) * 52; // Simulate orbital inclination

      // Jitter
      state.altitude += (Math.random() - 0.5) * 0.05;
      state.velocity += (Math.random() - 0.5) * 0.001;

      // Handle custom fault injection locally
      if (activeFault === "battery_drain") {
        state.battery = Math.max(0, state.battery - 2.5);
        state.solar = Math.max(0, state.solar - 4);
      } else if (activeFault === "solar_degrade") {
        state.solar = Math.max(0, state.solar - 3);
        state.battery = Math.max(0, state.battery - 0.2);
      } else if (activeFault === "orbit_decay") {
        state.altitude = Math.max(100, state.altitude - 1.2);
        state.velocity += 0.015; // gains speed temporarily as orbit collapses
      } else if (activeFault === "thruster_overheat") {
        state.temp = Math.min(100, state.temp + 1.8);
      } else {
        // Nominal battery simulation
        if (state.solar > 100) {
          state.battery = Math.min(100, state.battery + 0.1);
        } else {
          state.battery = Math.max(10, state.battery - 0.05);
        }
        // Nominal temperature tracking solar power
        const targetTemp = 20 + (state.solar / 10) + (Math.random() - 0.5) * 0.5;
        state.temp += (targetTemp - state.temp) * 0.1;
        // Nominal solar generation cycle
        state.solar = Math.max(0, 150 * Math.sin(state.lon * (Math.PI / 90)) + 50);
      }

      const formattedTime = new Date().toLocaleTimeString();
      const tickData = {
        time: formattedTime,
        altitude: state.altitude,
        velocity: state.velocity,
        temp: state.temp,
        solar: state.solar,
        battery_level: state.battery,
        battery_temp: state.temp,
        solar_power: state.solar,
        latitude: state.lat,
        longitude: state.lon,
      };

      setTelemetry(tickData);
      setHistory((prev) => [...prev.slice(-29), tickData]);

    }, 1000);

    return () => clearInterval(interval);
  }, [selectedSatellite, isWebSocketConnected, isSimulating, activeFault, isAuthenticated]);



  // Inject failure action
  const handleInjectFailure = async (failureType: string) => {
    const satName = SATELLITES.find(s => s.id === selectedSatellite)?.name || "Satellite";
    addEvent("warning", "SIMULATOR", `Injected fault trigger: [${failureType}] on ${satName}.`);
    
    setActiveFault(failureType);

    // If backend is active, dispatch payload with authorization header
    if (isWebSocketConnected) {
      try {
        await fetch(`${API_URL}/api/v1/simulator/inject`, {
          method: "POST",
          headers: { 
            "Content-Type": "application/json",
            "Authorization": `Bearer ${token}`
          },
          body: JSON.stringify({ satellite_id: selectedSatellite, fault: failureType }),
        });
      } catch (err) {
        console.error("Failed to post fault to simulator endpoint", err);
      }
    }
  };

  // Reset simulator
  const handleResetSimulator = async () => {
    const satName = SATELLITES.find(s => s.id === selectedSatellite)?.name || "Satellite";
    addEvent("info", "SIMULATOR", `Discharged all active fault registers. System reboot nominal for ${satName}.`);
    setActiveFault(null);

    // If backend is active, dispatch payload with authorization header
    if (isWebSocketConnected) {
      try {
        await fetch(`${API_URL}/api/v1/simulator/inject`, {
          method: "POST",
          headers: { 
            "Content-Type": "application/json",
            "Authorization": `Bearer ${token}`
          },
          body: JSON.stringify({ satellite_id: selectedSatellite, fault: null }),
        });
      } catch (err) {
        console.error("Failed to post fault reset to simulator endpoint", err);
      }
    }
  };

  // Mission management triggers
  const handleCreateMission = async (name: string, description: string, satelliteId: string | null) => {
    if (token) {
      try {
        const response = await fetch(`${API_URL}/api/v1/missions`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "Authorization": `Bearer ${token}`
          },
          body: JSON.stringify({
            name,
            description: description || null,
            status: "planned",
            satellite_id: satelliteId || null,
          }),
        });

        if (response.ok) {
          const newM = await response.json();
          const missionObj: Mission = {
            id: newM.id,
            name: newM.name,
            description: newM.description || "",
            status: newM.status as Mission["status"],
            satellite_id: satelliteId,
            start_date: newM.start_date.split("T")[0],
          };
          setMissions((prev) => [...prev, missionObj]);
          addEvent("info", "COMMAND", `Provisioned new mission profile: "${name}" in TimescaleDB.`);
          return;
        } else {
          const text = await response.text();
          addEvent("error", "COMMAND", `Failed to deploy mission: ${text}`);
        }
      } catch (err) {
        console.error("Failed to post mission", err);
      }
    }

    // Local fallback if no connection
    const newMission: Mission = {
      id: `m-${Date.now()}`,
      name,
      description,
      status: "planned",
      satellite_id: satelliteId,
      start_date: new Date().toISOString().substring(0, 10),
    };
    setMissions((prev) => [...prev, newMission]);
    addEvent("info", "COMMAND", `Provisioned local mission profile: "${name}" (Offline mode).`);
  };

  const handleDeleteMission = async (id: string) => {
    const m = missions.find(mission => mission.id === id);
    if (token && id.length === 36) { // database UUID has 36 characters
      try {
        const response = await fetch(`${API_URL}/api/v1/missions/${id}`, {
          method: "DELETE",
          headers: {
            "Authorization": `Bearer ${token}`
          }
        });
        if (response.ok) {
          setMissions((prev) => prev.filter((item) => item.id !== id));
          if (m) addEvent("warning", "COMMAND", `Terminated mission profile: "${m.name}" in TimescaleDB.`);
          return;
        } else {
          const text = await response.text();
          addEvent("error", "COMMAND", `Failed to delete mission from DB: ${text}`);
        }
      } catch (err) {
        console.error("Failed to delete mission", err);
      }
    }

    // Local fallback if offline or local id
    setMissions((prev) => prev.filter((item) => item.id !== id));
    if (m) addEvent("warning", "COMMAND", `Terminated local mission profile: "${m.name}".`);
  };

  if (loadingSession) {
    return (
      <div className="min-h-screen w-full flex items-center justify-center bg-[#09090b] font-mono text-zinc-500">
        <div className="flex flex-col items-center gap-2">
          <Loader2 className="h-6 w-6 animate-spin text-indigo-400" />
          <span>INITIALIZING COMMAND SYSTEM...</span>
        </div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <AuthScreen onAuthSuccess={handleAuthSuccess} />;
  }

  return (
    <div className="flex min-h-screen">
      {/* Sidebar navigation */}
      <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} onLogout={handleLogout} />

      {/* Main Workspace Frame */}
      <div className="flex-1 flex flex-col min-w-0">
        <Header 
          activeTab={activeTab}
          selectedSatellite={selectedSatellite}
          setSelectedSatellite={setSelectedSatellite}
          satellites={SATELLITES}
        />

        {/* Tab Views Panel */}
        <main className="flex-1 p-6 overflow-y-auto">
          {activeTab === "dashboard" && (
            <div className="space-y-6">
              {/* Telemetry Overview Cards */}
              <TelemetryGrid data={telemetry} />

              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                {/* Live charts */}
                <LiveCharts history={history} />

                {/* Event logs terminal */}
                <EventsLog events={events} clearEvents={() => setEvents([])} />
              </div>
            </div>
          )}

          {activeTab === "missions" && (
            <MissionsOverview 
              missions={missions}
              satellites={SATELLITES}
              onCreateMission={handleCreateMission}
              onDeleteMission={handleDeleteMission}
            />
          )}

          {activeTab === "controls" && (
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2 space-y-6">
                <div className="glass-panel border-[#27272a]/50 rounded-xl p-5 space-y-4">
                  <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider flex items-center gap-2">
                    <Cpu className="h-4.5 w-4.5 text-indigo-400" />
                    Telemetry Injector Overview
                  </h3>
                  <p className="text-xs text-zinc-400 font-mono leading-relaxed">
                    The Failure Injection Deck connects directly to the satellite telemetry streams. In order to test error triggers on the telemetry client, override parameters can be force-fed using the manual controls on the right panel.
                  </p>
                  <div className="border border-[#27272a]/50 rounded-lg p-4 bg-[#18181b]/30 space-y-2">
                    <div className="flex justify-between items-center text-xs font-mono">
                      <span className="text-zinc-500">BACKEND CONNECTION STATUS</span>
                      <span className={isWebSocketConnected ? "text-emerald-400" : "text-amber-400"}>
                        {isWebSocketConnected ? "UPLINK LIVE (PORT 8081)" : "FALLBACK LOCAL MODE"}
                      </span>
                    </div>
                    <div className="flex justify-between items-center text-xs font-mono">
                      <span className="text-zinc-500">ACTIVE OVERRIDES IN FEED</span>
                      <span className={activeFault ? "text-rose-400 font-bold" : "text-zinc-500"}>
                        {activeFault ? activeFault.toUpperCase() : "NONE (SYSTEM NOMINAL)"}
                      </span>
                    </div>
                    <div className="flex justify-between items-center text-xs font-mono">
                      <span className="text-zinc-500">LOGGED OPERATOR</span>
                      <span className="text-indigo-400">{username}</span>
                    </div>
                  </div>
                </div>

                <div className="glass-panel border-[#27272a]/50 rounded-xl p-5 space-y-4">
                  <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider flex items-center gap-2">
                    <Network className="h-4.5 w-4.5 text-indigo-400" />
                    Ingestion Architecture Status
                  </h3>
                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-4 font-mono text-[10px] text-zinc-400">
                    <div className="border border-[#27272a]/30 p-3 rounded bg-[#18181b]/20">
                      <span className="block text-zinc-600 mb-1">HTTP INGEST API</span>
                      <span className="text-emerald-400 font-bold">ONLINE</span>
                      <span className="block text-zinc-700 mt-0.5">POST /v1/telemetry</span>
                    </div>
                    <div className="border border-[#27272a]/30 p-3 rounded bg-[#18181b]/20">
                      <span className="block text-zinc-600 mb-1">REDIS STREAMER</span>
                      <span className="text-emerald-400 font-bold">ONLINE</span>
                      <span className="block text-zinc-700 mt-0.5">Pub/Sub broads</span>
                    </div>
                    <div className="border border-[#27272a]/30 p-3 rounded bg-[#18181b]/20">
                      <span className="block text-zinc-600 mb-1">NATS JETSTREAM</span>
                      <span className="text-zinc-600 font-bold">STANDBY</span>
                      <span className="block text-zinc-700 mt-0.5">Phase-12 target</span>
                    </div>
                  </div>
                </div>
              </div>

              <div className="lg:col-span-1">
                <SimControl 
                  onInjectFailure={handleInjectFailure}
                  onResetSimulator={handleResetSimulator}
                  isSimulating={isSimulating}
                />
              </div>
            </div>
          )}

          {activeTab === "console" && (
            <div className="space-y-6">
              <div className="glass-panel border-[#27272a]/50 rounded-xl p-5">
                <h3 className="text-sm font-bold text-white font-mono uppercase tracking-wider flex items-center gap-2 mb-3">
                  <Terminal className="h-4.5 w-4.5 text-indigo-400" />
                  Satellite Operational Log Console (ALL CHANNELS)
                </h3>
                <EventsLog events={events} clearEvents={() => setEvents([])} />
              </div>
            </div>
          )}

          {activeTab === "database" && (
            <div className="space-y-6">
              <div className="glass-panel border-[#27272a]/50 rounded-xl p-6">
                <div className="flex items-center gap-3 border-b border-[#27272a]/40 pb-4 mb-6">
                  <div className="p-2 rounded bg-indigo-500/10 text-indigo-400">
                    <Database className="h-6 w-6" />
                  </div>
                  <div>
                    <h3 className="text-md font-bold text-white font-mono uppercase tracking-wider">TimescaleDB Hypertable Analytics</h3>
                    <p className="text-[11px] text-zinc-500 font-mono">Timescale optimization and analytics registry</p>
                  </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                  <div className="border border-[#27272a]/50 rounded-xl p-4 bg-[#18181b]/20">
                    <span className="text-[10px] font-mono text-zinc-500 block uppercase mb-1">Telemetry Chunk Size</span>
                    <span className="text-xl font-bold font-mono text-white block">340 KB</span>
                    <p className="text-[10px] text-zinc-600 font-mono mt-2">Aggregated over 2 hour chunks.</p>
                  </div>
                  
                  <div className="border border-[#27272a]/50 rounded-xl p-4 bg-[#18181b]/20">
                    <span className="text-[10px] font-mono text-zinc-500 block uppercase mb-1">Time Bucket Intervals</span>
                    <span className="text-xl font-bold font-mono text-white block">10 Seconds</span>
                    <p className="text-[10px] text-zinc-600 font-mono mt-2">Configured time bucket rollup index.</p>
                  </div>

                  <div className="border border-[#27272a]/50 rounded-xl p-4 bg-[#18181b]/20">
                    <span className="text-[10px] font-mono text-zinc-500 block uppercase mb-1">Hypertable Status</span>
                    <span className="text-emerald-400 font-bold font-mono text-sm block flex items-center gap-1.5 mt-1">
                      <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 pulse-green"></span>
                      READY (PENDING MIGRATION)
                    </span>
                    <p className="text-[10px] text-zinc-600 font-mono mt-2">Ready to convert standard tables to hypertables.</p>
                  </div>
                </div>

                <div className="mt-8 border border-dashed border-[#27272a] rounded-lg p-6 text-center text-zinc-500 font-mono text-xs max-w-lg mx-auto">
                  <HelpCircle className="h-8 w-8 text-zinc-600 mx-auto mb-2" />
                  <span>TimescaleDB query cache validation is performed during compilation using SQLx macros. In Milestone 11, we will swap the vanilla PostgreSQL database engine to Timescale hypertables.</span>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
