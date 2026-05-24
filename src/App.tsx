import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

interface InstalledApp {
  name: string;
  path: string;
  category: string;
}

function AppIcon({ name, src }: { name: string; src?: string }) {
  if (src) {
    return (
      <img
        className="result-icon"
        src={`data:image/png;base64,${src}`}
        alt={name}
        draggable={false}
      />
    );
  }
  const letters = name
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w[0] ?? "")
    .join("")
    .toUpperCase();
  return <span className="result-icon result-icon--letter">{letters || "?"}</span>;
}

function App() {
  const [apps, setApps] = useState<InstalledApp[]>([]);
  const [icons, setIcons] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<InstalledApp[]>("get_apps").then((result) => {
      setApps(result);
      setLoading(false);
      if (result.length > 0) {
        const paths = result.map((a) => a.path);
        invoke<Record<string, string>>("get_icons", { paths }).then(setIcons);
      }
    });
  }, []);

  const results = query.trim()
    ? apps.filter(
        (app) =>
          app.name.toLowerCase().includes(query.toLowerCase()) ||
          app.category.toLowerCase().includes(query.toLowerCase())
      )
    : apps.slice(0, 7);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(
      `[data-index="${selectedIndex}"]`
    );
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const handleSelect = async (app: InstalledApp) => {
    await invoke("launch_app", { path: app.path });
  };

  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        const item = results[selectedIndex];
        if (item) await handleSelect(item);
      } else if (e.key === "Escape") {
        await getCurrentWindow().hide();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [results, selectedIndex]);

  return (
    <div className="launcher">
      <div className="search-bar">
        <svg
          className="search-icon"
          viewBox="0 0 20 20"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <circle
            cx="8.5"
            cy="8.5"
            r="5.5"
            stroke="currentColor"
            strokeWidth="1.6"
          />
          <path
            d="M13 13l3.5 3.5"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
          />
        </svg>
        <input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search apps…"
          spellCheck={false}
        />
      </div>

      <div className="divider" />

      <div className="results" ref={listRef}>
        {loading && <div className="empty-state">Loading…</div>}

        {!loading && results.length === 0 && (
          <div className="empty-state">No results for "{query}"</div>
        )}

        {!loading && results.length > 0 && (
          <>
            {!query && <div className="section-label">Suggested</div>}
            {results.map((app, i) => (
              <div
                key={app.path}
                data-index={i}
                className={`result-item${i === selectedIndex ? " selected" : ""}`}
                onMouseEnter={() => setSelectedIndex(i)}
                onClick={() => handleSelect(app)}
              >
                <AppIcon name={app.name} src={icons[app.path]} />
                <div className="result-text">
                  <span className="result-name">{app.name}</span>
                  {app.category && (
                    <span className="result-subtitle">{app.category}</span>
                  )}
                </div>
                {i === selectedIndex && (
                  <span className="result-enter-hint">↵</span>
                )}
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

export default App;
