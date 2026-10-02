import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer } from "recharts";
import { Cpu, MemoryStick, Activity } from "lucide-react";
import "./App.css";

interface DataPoint {
  time: string;
  cpu: number;
}

function App() {
  const [data, setData] = useState<DataPoint[]>([]);
  const [currentCpu, setCurrentCpu] = useState(0);
  const [currentMem, setCurrentMem] = useState(0);
  const [currentProcs, setCurrentProcs] = useState(0);

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const [cpu, mem, procs] = await invoke<[number, number, number]>("get_system_stats");
        setCurrentCpu(cpu);
        setCurrentMem(mem);
        setCurrentProcs(procs);
        
        setData((prev) => {
          const now = new Date().toLocaleTimeString('en-US', { hour12: false, minute: '2-digit', second: '2-digit' });
          const newPoint = { time: now, cpu: Number(cpu.toFixed(1)) };
          const newData = [...prev, newPoint];
          if (newData.length > 30) newData.shift();
          return newData;
        });
      } catch (err) {
        console.error(err);
      }
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div style={{ backgroundColor: "#0f172a", color: "#f8fafc", minHeight: "100vh", padding: "24px", fontFamily: "system-ui, sans-serif" }}>
      <h1 style={{ fontSize: "24px", fontWeight: "bold", marginBottom: "24px", color: "#38bdf8" }}>
        SysMon (Tauri + React PoC)
      </h1>
      
      <div style={{ display: "flex", gap: "24px", marginBottom: "32px" }}>
        <div style={{ backgroundColor: "#1e293b", padding: "24px", borderRadius: "12px", flex: 1, display: "flex", alignItems: "center", gap: "16px", boxShadow: "0 4px 6px -1px rgb(0 0 0 / 0.1)" }}>
          <Cpu size={48} color="#38bdf8" />
          <div>
            <div style={{ fontSize: "14px", color: "#94a3b8" }}>CPU Usage</div>
            <div style={{ fontSize: "36px", fontWeight: "bold" }}>{currentCpu.toFixed(1)}%</div>
          </div>
        </div>
        
        <div style={{ backgroundColor: "#1e293b", padding: "24px", borderRadius: "12px", flex: 1, display: "flex", alignItems: "center", gap: "16px", boxShadow: "0 4px 6px -1px rgb(0 0 0 / 0.1)" }}>
          <MemoryStick size={48} color="#a78bfa" />
          <div>
            <div style={{ fontSize: "14px", color: "#94a3b8" }}>Memory Used</div>
            <div style={{ fontSize: "36px", fontWeight: "bold" }}>{currentMem} MB</div>
          </div>
        </div>

        <div style={{ backgroundColor: "#1e293b", padding: "24px", borderRadius: "12px", flex: 1, display: "flex", alignItems: "center", gap: "16px", boxShadow: "0 4px 6px -1px rgb(0 0 0 / 0.1)" }}>
          <Activity size={48} color="#f43f5e" />
          <div>
            <div style={{ fontSize: "14px", color: "#94a3b8" }}>Processes</div>
            <div style={{ fontSize: "36px", fontWeight: "bold" }}>{currentProcs}</div>
          </div>
        </div>
      </div>

      <div style={{ backgroundColor: "#1e293b", padding: "24px", borderRadius: "12px", height: "300px" }}>
        <div style={{ marginBottom: "16px", fontWeight: "bold", color: "#e2e8f0" }}>Live CPU Telemetry</div>
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
            <defs>
              <linearGradient id="colorCpu" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#38bdf8" stopOpacity={0.8}/>
                <stop offset="95%" stopColor="#38bdf8" stopOpacity={0}/>
              </linearGradient>
            </defs>
            <XAxis dataKey="time" stroke="#64748b" fontSize={12} tickMargin={8} />
            <YAxis stroke="#64748b" fontSize={12} domain={[0, 100]} width={40} />
            <Tooltip 
              contentStyle={{ backgroundColor: "#0f172a", border: "1px solid #334155", borderRadius: "8px", color: "#f8fafc" }}
              itemStyle={{ color: "#38bdf8" }}
            />
            <Area type="monotone" dataKey="cpu" stroke="#38bdf8" strokeWidth={3} fillOpacity={1} fill="url(#colorCpu)" isAnimationActive={false} />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

export default App;
