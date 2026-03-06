import { useState } from "react";

const MENU_ITEMS = ["File", "Edit", "Selection", "View", "Go", "Run", "Terminal", "Help"];

function MenuButton({ label }: { label: string }) {
  const [hovered, setHovered] = useState(false);

  return (
    <button
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        background: hovered ? "rgba(255,255,255,0.08)" : "transparent",
        border: "none",
        color: "#9a9a9a",
        fontSize: 12,
        padding: "2px 8px",
        cursor: "default",
        borderRadius: 4,
        transition: "background 0.1s ease",
      }}
    >
      {label}
    </button>
  );
}

function NavButton({ children, title, pointer }: { children: React.ReactNode; title: string; pointer?: boolean }) {
  const [hovered, setHovered] = useState(false);

  return (
    <button
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      title={title}
      style={{
        background: hovered ? "rgba(255,255,255,0.08)" : "transparent",
        border: "none",
        color: "#555",
        fontSize: 14,
        padding: "2px 4px",
        cursor: pointer ? "pointer" : "default",
        borderRadius: 4,
        display: "flex",
        alignItems: "center",
        transition: "background 0.1s ease",
      }}
    >
      {children}
    </button>
  );
}

function BlinkLogo() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <rect x="2" y="2" width="12" height="12" rx="3" stroke="#888" strokeWidth="1.2" />
      <rect x="5" y="5" width="2" height="4" rx="0.5" fill="#888" />
      <rect x="9" y="5" width="2" height="4" rx="0.5" fill="#888" />
    </svg>
  );
}

function ChevronLeft() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
      <path d="M10.854 3.146a.5.5 0 010 .708L6.707 8l4.147 4.146a.5.5 0 01-.708.708l-4.5-4.5a.5.5 0 010-.708l4.5-4.5a.5.5 0 01.708 0z" />
    </svg>
  );
}

function ChevronRight() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
      <path d="M5.146 12.854a.5.5 0 010-.708L9.293 8 5.146 3.854a.5.5 0 11.708-.708l4.5 4.5a.5.5 0 010 .708l-4.5 4.5a.5.5 0 01-.708 0z" />
    </svg>
  );
}

function LayoutIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
      <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
      <line x1="5.5" y1="2.5" x2="5.5" y2="13.5" />
      <line x1="1.5" y1="6" x2="14.5" y2="6" />
    </svg>
  );
}

function SidebarIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
      <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
      <line x1="5.5" y1="2.5" x2="5.5" y2="13.5" />
    </svg>
  );
}

function PanelIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
      <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
      <line x1="1.5" y1="9.5" x2="14.5" y2="9.5" />
    </svg>
  );
}

export default function TitleBar() {
  return (
    <div
      style={{
        height: 30,
        background: "#181818",
        display: "flex",
        alignItems: "center",
        padding: "0 8px",
        borderBottom: "1px solid #232323",
        flexShrink: 0,
        userSelect: "none",
      }}
    >
      {/* Left: Logo + Menu + Nav arrows */}
      <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1 }}>
        <div style={{ padding: "0 6px", display: "flex", alignItems: "center" }}>
          <BlinkLogo />
        </div>
        {MENU_ITEMS.map((item) => (
          <MenuButton key={item} label={item} />
        ))}
        <div style={{ display: "flex", alignItems: "center", gap: 2, marginLeft: 4 }}>
          <NavButton title="Go back"><ChevronLeft /></NavButton>
          <NavButton title="Go forward"><ChevronRight /></NavButton>
        </div>
      </div>

      {/* Center: Title */}
      <span style={{ color: "#9a9a9a", fontSize: 12 }}>Blink</span>

      {/* Right: Layout buttons */}
      <div style={{ display: "flex", alignItems: "center", gap: 2, flex: 1, justifyContent: "flex-end" }}>
        <NavButton title="Toggle panel" pointer><PanelIcon /></NavButton>
        <NavButton title="Toggle primary side bar" pointer><SidebarIcon /></NavButton>
        <NavButton title="Change layout" pointer><LayoutIcon /></NavButton>
      </div>
    </div>
  );
}
