export interface Project {
  id: string;
  name: string;
  path: string;
  createdAt: string;
  sortOrder: number;
}

export interface TerminalInstance {
  id: string;
  projectId: string;
  projectName: string;
  projectPath: string;
  terminalName: string;
  status: TerminalStatus;
  launchedAt: string;
  activity: TerminalActivity;
  notificationEnabled: boolean;
  monitored: boolean;
  attached: boolean;
  lastIdleAt?: string;
}

export interface TabInfo {
  runtimeId: number[];
  title: string;
}

export interface ExternalTerminalInfo {
  hwnd: number;
  windowTitle: string;
  tabs: TabInfo[];
  processCwds: string[];
}

export type TerminalStatus = 'running' | 'stopped';

export type TerminalActivity = 'active' | 'idle';

export interface StatusChangedPayload {
  projectPath: string;
  status: TerminalActivity;
  terminalId: string;
  monitored: boolean;
  lastIdleAt?: string;
}

export interface ReminderSettings {
  enabled: boolean;
  intervalMinutes: number;
  maxRepeat: number;
}

export interface Settings {
  reminder: ReminderSettings;
  idleThresholdSecs: number;
  theme: string;
  language: string;
  autoAttachEnabled: boolean;
  alwaysOnTop: boolean;
}

export type ThemeType = 'dark' | 'light';

export interface ThemeFile {
  name: string;
  description: string;
  type: ThemeType;
  colors: Record<string, string>;
}

export interface ThemeListEntry {
  id: string;
  name: string;
  type: ThemeType;
  description: string;
}

/** Matches the serialized output of `LogEntry` from src-tauri/src/services/debug_log.rs (returned by the `get_debug_logs` Tauri command). */
export interface DebugLogEntry {
  timestamp: number;
  category: string;
  message: string;
}

export type Locale = 'en' | 'ko';

export interface Translations {
  // App
  app: {
    title: string;
    emptyState: string;
    onboarding: {
      step1: string;
      step2: string;
      step3: string;
      step4: string;
    };
  };
  // Sidebar
  sidebar: {
    projectList: string;
    noProjects: string;
    addProject: string;
    settings: string;
  };
  // TabBar
  tabBar: {
    all: string;
    pinTooltip: string;
  };
  // ProjectForm
  projectForm: {
    addTitle: string;
    editTitle: string;
    nameLabel: string;
    namePlaceholder: string;
    pathLabel: string;
    pathPlaceholder: string;
    browseTitle: string;
    browse: string;
    cancel: string;
    add: string;
    edit: string;
    adding: string;
    editing: string;
    nameRequired: string;
    pathRequired: string;
    folderSelectFailed: string;
  };
  // ProjectItem
  projectItem: {
    deleteConfirm: (name: string) => string;
    deleteTitle: string;
    launchTerminal: string;
    runningCount: (count: number) => string;
    edit: string;
    delete: string;
  };
  // TerminalList
  terminalList: {
    editName: string;
    notificationOn: string;
    notificationOff: string;
    newTerminal: string;
    statusActive: string;
    statusIdle: string;
    attachExternal: string;
    attached: string;
  };
  // Attach / Import
  attach: {
    importing: string;
    importSuccess: string;
    importFailed: string;
    wtSettingsNotFound: string;
  };
  // Settings
  settings: {
    title: string;
    theme: {
      title: string;
      select: string;
      dark: string;
      light: string;
      createCustom: string;
      edit: string;
      delete: string;
      editorDesc: string;
      saveAndApply: string;
      saving: string;
      cancel: string;
      loadFailed: string;
      invalidJson: string;
      nameTypeRequired: string;
      invalidType: string;
      invalidName: string;
      saveFailed: (error: string) => string;
      deleteFailed: (error: string) => string;
    };
    language: {
      title: string;
    };
    reminder: {
      title: string;
      enabled: string;
      enabledDesc: string;
      interval: string;
      maxRepeat: string;
      maxRepeatDesc: string;
    };
    autoAttach: {
      title: string;
      enabled: string;
      enabledDesc: string;
    };
    alwaysOnTop: {
      title: string;
      enabled: string;
      enabledDesc: string;
    };
    hooks: {
      title: string;
      registered: string;
      notRegistered: string;
      register: string;
      unregister: string;
      registering: string;
      unregistering: string;
      desc: string;
      unregisterConfirm: string;
      registerFailed: (error: string) => string;
      unregisterFailed: (error: string) => string;
    };
    debug: {
      title: string;
      refresh: string;
      autoRefreshOn: string;
      autoRefreshOff: string;
      copied: string;
      copy: string;
      clear: string;
      empty: string;
      loadFailed: string;
      clearFailed: string;
      count: (n: number) => string;
    };
  };
}
