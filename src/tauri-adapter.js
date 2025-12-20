// Tauri Adapter for Lvory
// This file bridges existing Electron IPC calls to Tauri commands
// allowing the React frontend to run unchanged in Tauri.

// Helper to access Tauri internals safely
const getTauri = () => window.__TAURI__;
const isTauri = () => !!window.__TAURI__;

// Generic invoker that maps Electron channels to Tauri commands
const tauriInvoke = async (cmd, args = {}) => {
  if (!isTauri()) return;
  try {
    // Convert 'singbox-start-core' (kebab-case) to 'singbox_start_core' (snake_case)
    // or keep as is depending on Rust command naming convention.
    // For now, we pass the channel name as the command.
    const commandName = cmd.replace(/-/g, '_'); 
    return await window.__TAURI__.core.invoke(commandName, args);
  } catch (error) {
    console.error(`Tauri IPC Error [${cmd}]:`, error);
    throw error;
  }
};

const setupTauriAdapter = () => {
  if (!isTauri()) {
    console.log('Not running in Tauri environment, skipping adapter.');
    return;
  }

  console.log('Initializing Tauri Adapter...');

  // Mock the electron object
  window.electron = {
    platform: window.__TAURI__.core.invoke('get_platform'), // Async in Tauri, might need adjustment

    // Window Management
    window: {
      control: (action) => tauriInvoke('window_control', { action }),
      action: (type) => tauriInvoke('window_action', { type }),
      minimize: () => window.__TAURI__.window.getCurrent().minimize(),
      maximize: () => window.__TAURI__.window.getCurrent().toggleMaximize(),
      close: () => window.__TAURI__.window.getCurrent().close(),
      show: () => window.__TAURI__.window.getCurrent().show(),
      quit: () => tauriInvoke('app_quit'),
      onVisibilityChange: (callback) => {
        // Implement Tauri event listener
        // window.__TAURI__.event.listen('window-visibility-change', ...)
      }
    },

    // Network
    getNetworkInterfaces: () => tauriInvoke('get_network_interfaces'),

    // Core Management (Sing-box)
    singbox: {
      checkInstalled: () => tauriInvoke('singbox_check_installed'),
      getVersion: () => tauriInvoke('singbox_get_version'),
      checkConfig: (configPath) => tauriInvoke('singbox_check_config', { configPath }),
      formatConfig: (configPath) => tauriInvoke('singbox_format_config', { configPath }),
      startCore: (options) => tauriInvoke('singbox_start_core', { options }),
      stopCore: () => tauriInvoke('singbox_stop_core'),
      getStatus: () => tauriInvoke('singbox_get_status'),
      getDetailedStatus: () => tauriInvoke('singbox_get_detailed_status'),
      
      onOutput: (callback) => {
        if (window.__TAURI__.event) {
          window.__TAURI__.event.listen('singbox-output', (event) => callback(event.payload));
        }
      },
      onExit: (callback) => {
        if (window.__TAURI__.event) {
          window.__TAURI__.event.listen('singbox-exit', (event) => callback(event.payload));
        }
      }
    },

    // Extended Core Manager for Settings UI
    coreManager: {
        getInstalledVersions: async () => {
            try {
                const versions = await tauriInvoke('singbox_get_installed_versions');
                return { success: true, versions };
            } catch (e) {
                return { success: false, error: e.toString() };
            }
        },
        getSingBoxReleases: async () => {
            try {
                const releases = await tauriInvoke('singbox_get_releases');
                return { success: true, releases };
            } catch (e) {
                return { success: false, error: e.toString() };
            }
        },
        switchVersion: async (version) => {
            try {
                await tauriInvoke('singbox_switch_version', { version });
                return { success: true };
            } catch (e) {
                return { success: false, error: e.toString() };
            }
        },
        deleteVersion: async (version) => {
            try {
                await tauriInvoke('singbox_delete_version', { version });
                return { success: true };
            } catch (e) {
                return { success: false, error: e.toString() };
            }
        },
        downloadVersion: async (version) => {
            try {
                const platform = await tauriInvoke('get_platform'); // linux, windows, macos
                const arch = await tauriInvoke('get_arch'); // x86_64, aarch64
                
                const releases = await tauriInvoke('singbox_get_releases');
                // Ensure version format matches tag (v1.2.3 vs 1.2.3)
                const release = releases.find(r => r.tag_name === version || r.tag_name === `v${version}`);
                
                if (!release) throw new Error("Version not found in releases");
        
                let osName = platform === 'macos' ? 'darwin' : platform;
                let archName = arch === 'x86_64' ? 'amd64' : (arch === 'aarch64' ? 'arm64' : arch);
                
                // Prioritize tar.gz or zip
                const asset = release.assets.find(a => 
                    a.name.toLowerCase().includes(osName) && 
                    a.name.toLowerCase().includes(archName) &&
                    (a.name.endsWith('.tar.gz') || a.name.endsWith('.zip'))
                );
        
                if (!asset) throw new Error(`No compatible asset found for ${osName}-${archName}`);
        
                await tauriInvoke('singbox_download_version', { version, url: asset.browser_download_url });
                return { success: true };
            } catch (e) {
                return { success: false, error: e.toString() };
            }
        }
    },

    // Profiles - Mapped to Subscription commands as Profiles=Subscriptions in new logic
    profiles: {
      getData: () => tauriInvoke('subscription_get_all').then(res => res.subscriptions),
      getFiles: async () => {
        try {
          const result = await tauriInvoke('subscription_get_all');
          if (result && result.subscriptions) {
            // 转换为前端期望的格式
            const files = Object.entries(result.subscriptions).map(([name, metadata]) => ({
              name: name,
              path: metadata.path || '',
              size: metadata.size || 'Unknown',
              createDate: metadata.createDate || metadata.lastUpdate || 'Unknown',
              protocol: metadata.protocol || 'singbox',
              status: metadata.status || 'active',
              isComplete: metadata.isComplete !== false,
              hasCache: metadata.hasCache || false,
              cacheInfo: metadata.cacheInfo || null
            }));
            return { success: true, files };
          }
          return { success: false, files: [], error: 'No subscriptions found' };
        } catch (error) {
          console.error('getFiles error:', error);
          return { success: false, files: [], error: error.toString() };
        }
      },
      getMetadata: async (fileName) => {
        try {
          const result = await tauriInvoke('subscription_get', { fileName });
          return { success: true, metadata: result };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      update: async (fileName) => {
        try {
          await tauriInvoke('subscription_update', { fileName, updates: {} });
          return { success: true };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      updateAll: async () => {
        try {
          // 获取所有订阅并逐个更新
          const allSubs = await tauriInvoke('subscription_get_all');
          if (allSubs && allSubs.subscriptions) {
            const updatePromises = Object.keys(allSubs.subscriptions).map(fileName =>
              tauriInvoke('subscription_update', { fileName, updates: {} }).catch(e => ({error: e}))
            );
            await Promise.all(updatePromises);
            return { success: true, message: 'All profiles updated' };
          }
          return { success: false, error: 'No profiles to update' };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      // 兼容 store.js 的 getStore API，虽然底层已改为 Rust Settings
      getStore: async () => {
          try {
              const settings = await tauriInvoke('get_settings');
              // 映射 Rust settings 到旧版 store 结构 (如果有差异)
              return { success: true, data: settings };
          } catch (e) {
              return { success: false, error: e.toString() };
          }
      },
      delete: async (fileName) => {
        try {
          await tauriInvoke('subscription_delete', { fileName });
          return { success: true };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      openInEditor: async (fileName) => {
        try {
          const result = await tauriInvoke('subscription_get', { fileName });
          if (result && result.path) {
            // 使用系统默认编辑器打开文件
            await window.__TAURI__.opener.openPath(result.path);
            return { success: true };
          }
          throw new Error('File path not found');
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      refreshLvorySync: async () => {
        try {
          // TODO: 实现 Lvory 协议的缓存刷新逻辑
          return { success: true, message: 'Lvory cache refreshed' };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      onData: (callback) => {
         // Implement polling or event listener if needed
      },
      onUpdated: (callback) => {
        if (window.__TAURI__.event) {
          window.__TAURI__.event.listen('profile-updated', (event) => callback(event.payload));
        }
      },
      onChanged: (callback) => {
        if (window.__TAURI__.event) {
          window.__TAURI__.event.listen('profiles-changed', (event) => callback(event.payload));
        }
      }
    },

    // Config
    config: {
      getPath: () => tauriInvoke('get_config_path'),
      setPath: async (configPath) => {
        try {
          await tauriInvoke('set_config_path', { configPath });
          return { success: true };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      getCurrent: () => tauriInvoke('get_current_config'),
    },
    
    // User Config
    userConfig: {
        get: () => tauriInvoke('get_user_config'),
        save: (config) => tauriInvoke('save_user_config', { config }),
    },

    // Traffic Stats
    trafficStats: {
        getCurrent: () => tauriInvoke('traffic_stats_get_current'),
        getHistory: (limit) => tauriInvoke('traffic_stats_get_history', { limit }),
    },

    // Node History
    nodeHistory: {
        get: (nodeId, limit) => tauriInvoke('get_node_history', { nodeId, limit }),
        getTotalTraffic: (nodeId) => tauriInvoke('get_node_total_traffic', { nodeId }),
    },

    // Subscriptions
    subscription: {
        add: (fileName, metadata) => tauriInvoke('subscription_add', { fileName, metadata }),
        get: (fileName) => tauriInvoke('subscription_get', { fileName }),
        getAll: () => tauriInvoke('subscription_get_all'),
        update: (fileName, updates) => tauriInvoke('subscription_update', { fileName, updates }),
        delete: (fileName) => tauriInvoke('subscription_delete', { fileName }),
    },
    
    // Settings
    settings: {
        get: () => tauriInvoke('get_settings'),
        save: (settings) => tauriInvoke('save_settings', { settings }),
        getAutoLaunch: () => tauriInvoke('get_auto_launch'),
        setAutoLaunch: (enable) => tauriInvoke('set_auto_launch', { enable }),
    },

    // Logs
    logs: {
        cleanup: (retentionDays) => tauriInvoke('perform_log_cleanup', { retentionDays }),
        getHistory: () => tauriInvoke('get_log_history'),
        getConnectionHistory: () => tauriInvoke('get_connection_log_history'),
        onMessage: (callback) => {
            if (window.__TAURI__.event) {
                window.__TAURI__.event.listen('log-message', (event) => callback(event.payload));
            }
        },
        onConnection: (callback) => {
            if (window.__TAURI__.event) {
                window.__TAURI__.event.listen('connection-log', (event) => callback(event.payload));
            }
        }
    },

    // System Proxy
    systemProxy: {
        set: (host, port) => tauriInvoke('set_system_proxy', { host, port }),
        clear: () => tauriInvoke('clear_system_proxy'),
    },

    // Traceroute
    traceroute: {
      execute: async (target) => {
        try {
          const result = await tauriInvoke('traceroute_execute', { target });
          return { success: true, hops: result };
        } catch (error) {
          return { success: false, error: error.toString() };
        }
      },
      validate: async (target) => {
        try {
          const result = await tauriInvoke('traceroute_validate', { target });
          return result;
        } catch (error) {
          console.error('Traceroute validation error:', error);
          return false;
        }
      }
    },

    // Fix Profile (for incomplete profiles)
    fixProfile: async (fileName) => {
      try {
        await tauriInvoke('profile_fix', { fileName });
        return { success: true };
      } catch (error) {
        return { success: false, error: error.toString() };
      }
    },

    // Utils
    openExternal: (url) => window.__TAURI__.opener.openUrl(url),
    clipboard: {
        writeText: (text) => window.__TAURI__.clipboard.writeText(text)
    },
    
    // Generic IPC fallback
    invoke: (channel, ...args) => tauriInvoke(channel, ...args),
    ipcRenderer: {
        on: (channel, callback) => {
            if (window.__TAURI__.event) {
                window.__TAURI__.event.listen(channel, (event) => callback(event, event.payload));
            }
        },
        invoke: (channel, ...args) => tauriInvoke(channel, ...args)
    }
  };
};

export default setupTauriAdapter;
