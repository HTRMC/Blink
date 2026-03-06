import { useState, useEffect } from "react";
import { useFileSystem } from "../hooks/useFileSystem";
import { get, set } from "idb-keyval";

interface RecentProject {
  name: string;
  handle: FileSystemDirectoryHandle;
}

const RECENT_PROJECTS_KEY = "blink-recent-projects";

async function getRecentProjects(): Promise<RecentProject[]> {
  return (await get<RecentProject[]>(RECENT_PROJECTS_KEY)) ?? [];
}

async function addRecentProject(project: RecentProject) {
  const existing = await getRecentProjects();
  const filtered = existing.filter((p) => p.name !== project.name);
  const updated = [project, ...filtered].slice(0, 10);
  await set(RECENT_PROJECTS_KEY, updated);
}

function ActionCard({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick?: () => void;
}) {
  const [hovered, setHovered] = useState(false);

  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        width: 160,
        height: 80,
        background: hovered ? "#2a2a2a" : "#232323",
        border: "1px solid #333",
        borderRadius: 6,
        cursor: "pointer",
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-start",
        justifyContent: "center",
        padding: "16px 18px",
        gap: 10,
        transition: "background 0.15s ease",
      }}
    >
      <span style={{ color: "#888", fontSize: 18, lineHeight: 1 }}>{icon}</span>
      <span
        style={{
          color: "#ccc",
          fontSize: 13,
          fontWeight: 400,
        }}
      >
        {label}
      </span>
    </button>
  );
}

function FolderOpenIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 16 16" fill="none">
      <path
        d="M1.5 2h4.333l1.334 2H14.5v9.5h-13V2z"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinejoin="round"
        fill="none"
      />
      <path d="M1.5 6h13" stroke="currentColor" strokeWidth="1" />
    </svg>
  );
}

function CloneIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 16 16" fill="none">
      <rect
        x="3.5"
        y="1.5"
        width="9"
        height="9"
        rx="1"
        stroke="currentColor"
        strokeWidth="1"
      />
      <rect
        x="5.5"
        y="5.5"
        width="9"
        height="9"
        rx="1"
        stroke="currentColor"
        strokeWidth="1"
        fill="#181818"
      />
    </svg>
  );
}

function SshIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 16 16" fill="none">
      <rect
        x="1.5"
        y="3.5"
        width="13"
        height="9"
        rx="1"
        stroke="currentColor"
        strokeWidth="1"
      />
      <path
        d="M4 8l2 2M4 8l2-2M8 10h3"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default function WelcomePage() {
  const { openDirectory } = useFileSystem();
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    getRecentProjects().then(setRecentProjects);
  }, []);

  const handleOpenProject = async () => {
    await openDirectory();
  };

  const handleOpenRecent = async (project: RecentProject) => {
    try {
      const perm = await project.handle.requestPermission({ mode: "readwrite" });
      if (perm === "granted") {
        await openDirectory(project.handle);
      }
    } catch {
      // Permission denied or handle invalid
    }
  };

  const visibleProjects = showAll
    ? recentProjects
    : recentProjects.slice(0, 5);

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "#181818",
      }}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 32,
        }}
      >
        {/* Action cards */}
        <div style={{ display: "flex", gap: 16 }}>
          <ActionCard
            icon={<FolderOpenIcon />}
            label="Open project"
            onClick={handleOpenProject}
          />
          <ActionCard icon={<CloneIcon />} label="Clone repo" />
          <ActionCard icon={<SshIcon />} label="Connect via SSH" />
        </div>

        {/* Recent projects */}
        {recentProjects.length > 0 && (
          <div style={{ width: 520 }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: 8,
              }}
            >
              <span style={{ fontSize: 12, color: "#888" }}>
                Recent projects
              </span>
              {recentProjects.length > 5 && (
                <button
                  onClick={() => setShowAll(!showAll)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "#888",
                    fontSize: 12,
                    cursor: "pointer",
                    padding: 0,
                  }}
                >
                  {showAll
                    ? "Show less"
                    : `View all (${recentProjects.length})`}
                </button>
              )}
            </div>
            {visibleProjects.map((project) => (
              <RecentProjectRow
                key={project.name}
                project={project}
                onClick={() => handleOpenRecent(project)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function RecentProjectRow({
  project,
  onClick,
}: {
  project: RecentProject;
  onClick: () => void;
}) {
  const [hovered, setHovered] = useState(false);

  return (
    <div
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        padding: "6px 8px",
        borderRadius: 4,
        cursor: "pointer",
        background: hovered ? "rgba(255,255,255,0.04)" : "transparent",
        transition: "background 0.1s ease",
      }}
    >
      <span style={{ fontSize: 13, color: "#ccc" }}>{project.name}</span>
    </div>
  );
}
