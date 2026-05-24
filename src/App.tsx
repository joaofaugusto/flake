import { useState, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

interface LauncherItem {
  id: string;
  name: string;
  subtitle: string;
  icon: string;
  category: string;
}

const ALL_ITEMS: LauncherItem[] = [
  { id: "calc",     name: "Calculator",    subtitle: "Perform calculations",       icon: "🧮", category: "Utilities"     },
  { id: "browser",  name: "Browser",       subtitle: "Browse the internet",        icon: "🌐", category: "Internet"      },
  { id: "terminal", name: "Terminal",      subtitle: "Command line interface",     icon: "⌨️", category: "Developer"     },
  { id: "files",    name: "File Explorer", subtitle: "Browse and manage files",    icon: "📁", category: "System"        },
  { id: "settings", name: "Settings",      subtitle: "System preferences",         icon: "⚙️", category: "System"        },
  { id: "notes",    name: "Notes",         subtitle: "Quick notes and memos",      icon: "📝", category: "Productivity"  },
  { id: "mail",     name: "Mail",          subtitle: "Email client",               icon: "✉️", category: "Communication" },
  { id: "calendar", name: "Calendar",      subtitle: "Events and schedule",        icon: "📅", category: "Productivity"  },
  { id: "photos",   name: "Photos",        subtitle: "View and organize photos",   icon: "🖼️", category: "Media"         },
  { id: "music",    name: "Music",         subtitle: "Play music",                 icon: "🎵", category: "Media"         },
  { id: "maps",     name: "Maps",          subtitle: "Navigation and directions",  icon: "🗺️", category: "Utilities"     },
  { id: "lock",     name: "Lock Screen",   subtitle: "Lock your computer",         icon: "🔒", category: "System"        },
];

const RECENT = ALL_ITEMS.slice(0, 5);

function App() {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const results = query.trim()
    ? ALL_ITEMS.filter(
        (item) =>
          item.name.toLowerCase().includes(query.toLowerCase()) ||
          item.subtitle.toLowerCase().includes(query.toLowerCase()) ||
          item.category.toLowerCase().includes(query.toLowerCase())
      )
    : RECENT;

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Escape") {
        await getCurrentWindow().hide();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [results.length]);

  return (
    <div className="launcher">
      <div className="search-bar">
        <svg className="search-icon" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
          <circle cx="8.5" cy="8.5" r="5.5" stroke="currentColor" strokeWidth="1.6" />
          <path d="M13 13l3.5 3.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
        </svg>
        <input
          ref={inputRef}
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search apps and commands…"
          spellCheck={false}
        />
      </div>

      <div className="divider" />

      <div className="results" ref={listRef}>
        {results.length === 0 ? (
          <div className="empty-state">No results for "{query}"</div>
        ) : (
          <>
            {!query && <div className="section-label">Recents</div>}
            {results.map((item, i) => (
              <div
                key={item.id}
                data-index={i}
                className={`result-item${i === selectedIndex ? " selected" : ""}`}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <span className="result-icon">{item.icon}</span>
                <div className="result-text">
                  <span className="result-name">{item.name}</span>
                  <span className="result-subtitle">{item.subtitle}</span>
                </div>
                <span className="result-category">{item.category}</span>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

export default App;
