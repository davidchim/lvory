// 版本信息常量定义
const VERSION_INFO = {
  APP_VERSION: '0.1.7', // 默认版本号
  APP_NAME: 'lvory',
  APP_DESCRIPTION: '基于Sing-Box内核的通用桌面GUI客户端',
  LICENSE: 'Apache-2.0',
  WEBSITE: 'https://github.com/sxueck/lvory',
  BUILD_DATE: '20240101', // 默认构建日期，将被CI替换
};

// 尝试从Electron环境获取版本信息
const initVersionInfo = async () => {
    if (window.electron) {
        try {
            // 获取应用版本
            const versionResult = await window.electron.invoke('get-app-version');
            if (versionResult && versionResult.success) {
                VERSION_INFO.APP_VERSION = versionResult.version || VERSION_INFO.APP_VERSION;
            }
            
            // 获取构建日期
            const buildDateResult = await window.electron.invoke('get-build-date');
            if (buildDateResult && buildDateResult.success) {
                VERSION_INFO.BUILD_DATE = buildDateResult.buildDate || VERSION_INFO.BUILD_DATE;
            }
        } catch (error) {
            console.error('无法获取应用版本信息:', error);
        }
    }
};

// 初始化版本信息
initVersionInfo();

// 获取应用版本信息
const getAppVersion = () => VERSION_INFO.APP_VERSION;

// 获取构建日期
const getBuildDate = () => VERSION_INFO.BUILD_DATE;

// 获取应用名称
const getAppName = () => VERSION_INFO.APP_NAME;

// 获取完整版本信息对象
const getVersionInfo = () => ({...VERSION_INFO});

// 获取格式化的关于信息
const getAboutInfo = async () => {
    let coreVersion = 'Not Installed';

    // 尝试更新应用版本信息
    await initVersionInfo();

    // 尝试获取内核版本
    if (window.electron && window.electron.singbox && window.electron.singbox.getVersion) {
        try {
            const result = await window.electron.singbox.getVersion();
            if (result.success) {
                coreVersion = result.version || 'Unknown';
            }
        } catch (error) {
            console.error('获取内核版本失败:', error);
        }
    }

    // 获取构建日期
    const buildDate = await window.electron.invoke('get-build-date');
    const buildDateValue = buildDate && buildDate.success ? buildDate.buildDate : 'Unknown';

    return {
        ...VERSION_INFO,
        CORE_VERSION: coreVersion,
        BUILD_DATE: buildDateValue
    };
};

export {
  getAppVersion,
  getBuildDate,
  getAppName,
  getVersionInfo,
  getAboutInfo,
}; 