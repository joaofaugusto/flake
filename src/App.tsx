import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
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
  return (
    <span className="result-icon result-icon--letter">{letters || "?"}</span>
  );
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

  // Only show results when the user has typed something
  const hasQuery = query.trim().length > 0;
  const results = hasQuery
    ? apps.filter(
        (app) =>
          app.name.toLowerCase().includes(query.toLowerCase()) ||
          app.category.toLowerCase().includes(query.toLowerCase()),
      )
    : [];

  // Dynamically resize the window to match content height
  useEffect(() => {
    const launcher = document.querySelector<HTMLElement>(".launcher");
    if (launcher) {
      const height = launcher.clientHeight;
      getCurrentWindow().setSize(new LogicalSize(680, height));
    }
  }, [results, query, loading]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(
      `[data-index="${selectedIndex}"]`,
    );
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const handleSelect = async (app: InstalledApp) => {
    invoke("track_launch", { path: app.path });
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
            cx="9.5"
            cy="9.5"
            r="6"
            stroke="currentColor"
            strokeWidth="1.8"
          />
          <path
            d="M14 14l4 4"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
          />
        </svg>
        <input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Spotlight Search"
          spellCheck={false}
        />
      </div>

      {hasQuery && <div className="divider" />}

      <div className="results" ref={listRef}>
        {loading && hasQuery && <div className="empty-state">Searching...</div>}

        {!loading && hasQuery && results.length === 0 && (
          <div className="empty-state">No results found for "{query}"</div>
        )}

        {!loading && results.length > 0 && (
          <>
            <div className="section-label">Applications</div>
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
                  <span className="shortcut-badge">return</span>
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
